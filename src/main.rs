#![allow(non_snake_case)]

use VoxelTest::run;

// TODO: Add chunks logic
fn main() {
    pollster::block_on(run());
}
