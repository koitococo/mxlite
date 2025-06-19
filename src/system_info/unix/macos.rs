#![cfg(target_os = "macos")]

use super::BlkInfo;

pub(super) fn get_blk_info() -> Vec<BlkInfo> {
  vec![] // TODO: implement for non-linux systems
}