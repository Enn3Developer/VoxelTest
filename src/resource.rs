use crate::assets::Res;
use crate::model::{Material, Mesh, ObjModel, ModelVertex};
use crate::texture::Texture;
use anyhow::Result;
use std::io::{BufReader, Cursor};
use std::path::Path;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{BindGroupLayout, BufferUsages, Device, Queue};

pub fn load_string<P: AsRef<Path>>(file_name: P) -> Result<String> {
    let res = Res::get(&format!("res/{}", file_name.as_ref().to_string_lossy())).unwrap();
    let txt = String::from_utf8(res.data.into())?;

    Ok(txt)
}

pub fn load_binary(file_name: &str) -> Result<Vec<u8>> {
    let res = Res::get(&format!("res/{file_name}")).unwrap();
    let data = res.data.into();

    Ok(data)
}

pub fn load_texture(
    file_name: &str,
    device: &Device,
    queue: &Queue,
    is_normal_map: bool,
) -> Result<Texture> {
    let data = load_binary(file_name)?;
    Texture::from_bytes(device, queue, &data, file_name, is_normal_map)
}

pub fn load_model(
    file_name: &str,
    device: &Device,
    queue: &Queue,
    layout: &BindGroupLayout,
) -> Result<ObjModel> {
    let obj_text = load_string(file_name)?;
    let obj_cursor = Cursor::new(obj_text);
    let mut obj_reader = BufReader::new(obj_cursor);

    let (models, obj_materials) =
        tobj::load_obj_buf(&mut obj_reader, &tobj::GPU_LOAD_OPTIONS, |p| {
            let mat_text = load_string(p).unwrap();
            tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
        })?;

    let mut materials = vec![];
    for m in obj_materials? {
        let diffuse_texture =
            load_texture(&m.diffuse_texture.unwrap(), device, queue, false)?;

        materials.push(Material::new(device, &m.name, diffuse_texture, layout));
    }

    let meshes = models
        .into_iter()
        .map(|m| {
            let vertices = (0..m.mesh.positions.len() / 3)
                .map(|i| ModelVertex {
                    position: [
                        m.mesh.positions[i * 3],
                        m.mesh.positions[i * 3 + 1],
                        m.mesh.positions[i * 3 + 2],
                    ],
                    tex_coords: [m.mesh.texcoords[i * 2], m.mesh.texcoords[i * 2 + 1]],
                })
                .collect::<Vec<ModelVertex>>();

            let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
                label: Some(&format!("{:?} Vertex Buffer", file_name)),
                contents: bytemuck::cast_slice(&vertices),
                usage: BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
                label: Some(&format!("{:?} Index Buffer", file_name)),
                contents: bytemuck::cast_slice(&m.mesh.indices),
                usage: BufferUsages::INDEX,
            });

            Mesh {
                name: file_name.to_string(),
                vertex_buffer,
                index_buffer,
                num_elements: m.mesh.indices.len() as u32,
                material: m.mesh.material_id.unwrap_or(0),
            }
        })
        .collect();

    Ok(ObjModel { meshes, materials })
}
