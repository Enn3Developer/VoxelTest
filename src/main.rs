#![allow(non_snake_case)]

use VoxelTest::run;

fn main() {
    pollster::block_on(run());
}
