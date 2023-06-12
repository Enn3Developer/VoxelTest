use glam::{Mat4, Vec3};
use std::mem;

pub struct FrustumCuller {
    nx_x: f32,
    nx_y: f32,
    nx_z: f32,
    nx_w: f32,
    px_x: f32,
    px_y: f32,
    px_z: f32,
    px_w: f32,
    ny_x: f32,
    ny_y: f32,
    ny_z: f32,
    ny_w: f32,
    py_x: f32,
    py_y: f32,
    py_z: f32,
    py_w: f32,
    nz_x: f32,
    nz_y: f32,
    nz_z: f32,
    nz_w: f32,
    pz_x: f32,
    pz_y: f32,
    pz_z: f32,
    pz_w: f32,
}

pub struct Aabb {
    min: Vec3,
    max: Vec3,
}

impl Aabb {
    #[inline]
    pub fn from_params(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }
}

impl FrustumCuller {
    pub fn from_matrix(m: Mat4) -> Self {
        let mut culler: Self = unsafe { mem::zeroed() };

        culler.nx_x = m.x_axis.w + m.x_axis.x;
        culler.nx_y = m.y_axis.w + m.y_axis.x;
        culler.nx_z = m.z_axis.w + m.z_axis.x;
        culler.nx_w = m.w_axis.w + m.w_axis.x;
        culler.px_x = m.x_axis.w - m.x_axis.x;
        culler.px_y = m.y_axis.w - m.y_axis.x;
        culler.px_z = m.z_axis.w - m.z_axis.x;
        culler.px_w = m.w_axis.w - m.w_axis.x;
        culler.ny_x = m.x_axis.w + m.x_axis.y;
        culler.ny_y = m.y_axis.w + m.y_axis.y;
        culler.ny_z = m.z_axis.w + m.z_axis.y;
        culler.ny_w = m.w_axis.w + m.w_axis.y;
        culler.py_x = m.x_axis.w - m.x_axis.y;
        culler.py_y = m.y_axis.w - m.y_axis.y;
        culler.py_z = m.z_axis.w - m.z_axis.y;
        culler.py_w = m.w_axis.w - m.w_axis.y;
        culler.nz_x = m.x_axis.w + m.x_axis.z;
        culler.nz_y = m.y_axis.w + m.y_axis.z;
        culler.nz_z = m.z_axis.w + m.z_axis.z;
        culler.nz_w = m.w_axis.w + m.w_axis.z;
        culler.pz_x = m.x_axis.w - m.x_axis.z;
        culler.pz_y = m.y_axis.w - m.y_axis.z;
        culler.pz_z = m.z_axis.w - m.z_axis.z;
        culler.pz_w = m.w_axis.w - m.w_axis.z;

        culler
    }

    pub fn test_bounding_box(&self, aab: &Aabb) -> bool {
        self.nx_x
            * if self.nx_x < 0.0 {
                aab.min.x
            } else {
                aab.max.x
            }
            + self.nx_y
                * if self.nx_y < 0.0 {
                    aab.min.y
                } else {
                    aab.max.y
                }
            + self.nx_z
                * if self.nx_z < 0.0 {
                    aab.min.z
                } else {
                    aab.max.z
                }
            >= -self.nx_w
            && self.px_x
                * if self.px_x < 0.0 {
                    aab.min.x
                } else {
                    aab.max.x
                }
                + self.px_y
                    * if self.px_y < 0.0 {
                        aab.min.y
                    } else {
                        aab.max.y
                    }
                + self.px_z
                    * if self.px_z < 0.0 {
                        aab.min.z
                    } else {
                        aab.max.z
                    }
                >= -self.px_w
            && self.ny_x
                * if self.ny_x < 0.0 {
                    aab.min.x
                } else {
                    aab.max.x
                }
                + self.ny_y
                    * if self.ny_y < 0.0 {
                        aab.min.y
                    } else {
                        aab.max.y
                    }
                + self.ny_z
                    * if self.ny_z < 0.0 {
                        aab.min.z
                    } else {
                        aab.max.z
                    }
                >= -self.ny_w
            && self.py_x
                * if self.py_x < 0.0 {
                    aab.min.x
                } else {
                    aab.max.x
                }
                + self.py_y
                    * if self.py_y < 0.0 {
                        aab.min.y
                    } else {
                        aab.max.y
                    }
                + self.py_z
                    * if self.py_z < 0.0 {
                        aab.min.z
                    } else {
                        aab.max.z
                    }
                >= -self.py_w
            && self.nz_x
                * if self.nz_x < 0.0 {
                    aab.min.x
                } else {
                    aab.max.x
                }
                + self.nz_y
                    * if self.nz_y < 0.0 {
                        aab.min.y
                    } else {
                        aab.max.y
                    }
                + self.nz_z
                    * if self.nz_z < 0.0 {
                        aab.min.z
                    } else {
                        aab.max.z
                    }
                >= -self.nz_w
            && self.pz_x
                * if self.pz_x < 0.0 {
                    aab.min.x
                } else {
                    aab.max.x
                }
                + self.pz_y
                    * if self.pz_y < 0.0 {
                        aab.min.y
                    } else {
                        aab.max.y
                    }
                + self.pz_z
                    * if self.pz_z < 0.0 {
                        aab.min.z
                    } else {
                        aab.max.z
                    }
                >= -self.pz_w
    }
}
