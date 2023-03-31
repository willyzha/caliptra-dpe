// Licensed under the Apache-2.0 license.
use super::CommandExecution;
use crate::{
    dpe_instance::DpeInstance,
    response::{DpeErrorCode, NewHandleResp, Response},
    HANDLE_SIZE,
};
use core::mem::size_of;
use crypto::Crypto;

#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(zerocopy::AsBytes, zerocopy::FromBytes))]
pub struct RotateCtxCmd {
    handle: [u8; HANDLE_SIZE],
    flags: u32,
    target_locality: u32,
}

impl TryFrom<&[u8]> for RotateCtxCmd {
    type Error = DpeErrorCode;

    fn try_from(raw: &[u8]) -> Result<Self, Self::Error> {
        if raw.len() < size_of::<RotateCtxCmd>() {
            return Err(DpeErrorCode::InvalidArgument);
        }

        let mut handle = [0; HANDLE_SIZE];
        handle.copy_from_slice(&raw[0..HANDLE_SIZE]);

        let raw = &raw[HANDLE_SIZE..];
        Ok(RotateCtxCmd {
            handle,
            flags: u32::from_le_bytes(raw[0..4].try_into().unwrap()),
            target_locality: u32::from_le_bytes(raw[4..8].try_into().unwrap()),
        })
    }
}

impl<C: Crypto> CommandExecution<C> for RotateCtxCmd {
    fn execute(&self, dpe: &mut DpeInstance<C>, locality: u32) -> Result<Response, DpeErrorCode> {
        if !dpe.support.rotate_context {
            return Err(DpeErrorCode::InvalidCommand);
        }
        let idx = dpe
            .get_active_context_pos(&self.handle, locality)
            .ok_or(DpeErrorCode::InvalidHandle)?;

        // Make sure the command is coming from the right locality.
        if dpe.contexts[idx].locality != locality {
            return Err(DpeErrorCode::InvalidHandle);
        }

        let new_handle = dpe.generate_new_handle()?;
        dpe.contexts[idx].handle = new_handle;
        Ok(Response::RotateCtx(NewHandleResp { handle: new_handle }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        commands::{Command, CommandHdr, InitCtxCmd},
        dpe_instance::{
            tests::{SIMULATION_HANDLE, TEST_HANDLE, TEST_LOCALITIES},
            Support,
        },
    };
    use crypto::OpensslCrypto;
    use zerocopy::{AsBytes, FromBytes};

    const TEST_ROTATE_CTX_CMD: RotateCtxCmd = RotateCtxCmd {
        flags: 0x1234_5678,
        handle: TEST_HANDLE,
        target_locality: 0x9876_5432,
    };

    #[test]
    fn try_from_rotate_ctx() {
        let command_bytes = TEST_ROTATE_CTX_CMD.as_bytes();
        assert_eq!(
            RotateCtxCmd::read_from_prefix(command_bytes).unwrap(),
            RotateCtxCmd::try_from(command_bytes).unwrap(),
        );
    }

    #[test]
    fn test_deserialize_rotate_context() {
        let mut command = CommandHdr::new(Command::RotateCtx(TEST_ROTATE_CTX_CMD))
            .as_bytes()
            .to_vec();
        command.extend(TEST_ROTATE_CTX_CMD.as_bytes());
        assert_eq!(
            Ok(Command::RotateCtx(TEST_ROTATE_CTX_CMD)),
            Command::deserialize(&command)
        );
    }

    #[test]
    fn test_slice_to_rotate_ctx() {
        let invalid_argument: Result<RotateCtxCmd, DpeErrorCode> =
            Err(DpeErrorCode::InvalidArgument);

        // Test if too small.
        assert_eq!(
            invalid_argument,
            RotateCtxCmd::try_from([0u8; size_of::<RotateCtxCmd>() - 1].as_slice())
        );

        assert_eq!(
            TEST_ROTATE_CTX_CMD,
            RotateCtxCmd::try_from(TEST_ROTATE_CTX_CMD.as_bytes()).unwrap()
        );
    }

    #[test]
    fn test_rotate_context() {
        let mut dpe =
            DpeInstance::<OpensslCrypto>::new(Support::default(), &TEST_LOCALITIES).unwrap();
        // Make sure it returns an error if the command is marked unsupported.
        assert_eq!(
            Err(DpeErrorCode::InvalidCommand),
            RotateCtxCmd {
                handle: DpeInstance::<OpensslCrypto>::DEFAULT_CONTEXT_HANDLE,
                flags: 0,
                target_locality: 0
            }
            .execute(&mut dpe, TEST_LOCALITIES[0])
        );

        // Make a new instance that supports RotateContext.
        let mut dpe = DpeInstance::<OpensslCrypto>::new(
            Support {
                rotate_context: true,
                ..Support::default()
            },
            &TEST_LOCALITIES,
        )
        .unwrap();
        InitCtxCmd::new_use_default()
            .execute(&mut dpe, TEST_LOCALITIES[0])
            .unwrap();

        // Invalid handle.
        assert_eq!(
            Err(DpeErrorCode::InvalidHandle),
            RotateCtxCmd {
                handle: TEST_HANDLE,
                flags: 0,
                target_locality: 0
            }
            .execute(&mut dpe, TEST_LOCALITIES[0])
        );

        // Wrong locality.
        assert_eq!(
            Err(DpeErrorCode::InvalidHandle),
            RotateCtxCmd {
                handle: DpeInstance::<OpensslCrypto>::DEFAULT_CONTEXT_HANDLE,
                flags: 0,
                target_locality: 0
            }
            .execute(&mut dpe, TEST_LOCALITIES[1])
        );

        // Rotate default handle.
        assert_eq!(
            Ok(Response::RotateCtx(NewHandleResp {
                handle: SIMULATION_HANDLE
            })),
            RotateCtxCmd {
                handle: DpeInstance::<OpensslCrypto>::DEFAULT_CONTEXT_HANDLE,
                flags: 0,
                target_locality: 0
            }
            .execute(&mut dpe, TEST_LOCALITIES[0])
        );
    }
}
