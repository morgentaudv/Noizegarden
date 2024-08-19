// Rustの型変換イディオム
// https://qiita.com/legokichi/items/0f1c592d46a9aaf9a0ea#u8---str
// https://docs.rs/clap/latest/clap/_derive/_tutorial/chapter_3/index.html

use carg::parse_command_arguments;

#[macro_use]
extern crate static_assertions;

pub mod carg;
pub mod math;
pub mod wave;

fn main() -> anyhow::Result<()> {
    parse_command_arguments()
}
