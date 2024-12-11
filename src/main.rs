// Rustの型変換イディオム
// https://qiita.com/legokichi/items/0f1c592d46a9aaf9a0ea#u8---str
// https://docs.rs/clap/latest/clap/_derive/_tutorial/chapter_3/index.html

use carg::parse_command_arguments;

#[macro_use]
extern crate static_assertions;

pub mod carg;
pub mod math;
pub mod wave;

pub mod device;

fn main() -> anyhow::Result<()> {
    // @todo 24-12-05 後でParseを非同期で行うなど。
    let container = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(parse_command_arguments())?;
    container.process()?;

    Ok(())
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
