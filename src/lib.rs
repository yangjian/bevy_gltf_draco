use bevy::asset::LoadContext;
use bevy::gltf::extensions::GltfExtensionHandlers;
use bevy::gltf::{GltfAssetLabel, GltfLoaderSettings, preprocess_mesh};
use bevy::mesh::MeshVertexAttribute;
use bevy::platform::collections::HashSet;
use bevy::{
    app::{App, Plugin},
    gltf::extensions::ErasedGltfExtensionHandler,
    log::error,
    mesh::Mesh,
};
use bevy::{
    gltf::{
        extensions::GltfExtensionHandler,
        gltf::{Gltf as JsonGltf, Primitive},
    },
    platform::collections::HashMap,
};

use crate::khr_draco_mesh_compression::DracoExtension;

mod khr_draco_mesh_compression;

#[derive(Default, Clone)]
struct GltfDracoDecoderExtensionHandler;

impl GltfExtensionHandler for GltfDracoDecoderExtensionHandler {
    fn dyn_clone(&self) -> Box<dyn ErasedGltfExtensionHandler> {
        Box::new((*self).clone())
    }

    async fn on_gltf_primitive(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf: &JsonGltf,
        gltf_mesh: &gltf::Mesh<'_>,
        gltf_primitive: &Primitive<'_>,
        buffer_data: &[Vec<u8>],
        settings: &GltfLoaderSettings,
        custom_vertex_attributes: &HashMap<Box<str>, MeshVertexAttribute>,
        convert_coordinates: bool,
        meshes_on_skinned_nodes: &HashSet<usize>,
        meshes_on_non_skinned_nodes: &HashSet<usize>,
        user_mesh: &mut Option<Mesh>,
    ) {
        let Some(draco_extension) = DracoExtension::parse(load_context, gltf, gltf_primitive)
        else {
            error!("fail to make draco_extension");
            return;
        };
        let Some((config, decode_data)) = draco_extension.decode_mesh(gltf, buffer_data).await
        else {
            error!("fail to make draco mesh");
            return;
        };

        let Some(draco_primitive_document) =
            draco_extension.build_document(gltf_primitive, &config)
        else {
            error!("fail to build draco primitive");
            return;
        };

        let draco_primitive = DracoExtension::primitive(&draco_primitive_document);

        let primitive_label = GltfAssetLabel::Primitive {
            mesh: gltf_mesh.index(),
            primitive: gltf_primitive.index(),
        };

        if let Ok(mesh) = preprocess_mesh(
            gltf_mesh,
            &draco_primitive,
            &primitive_label,
            settings,
            &decode_data,
            custom_vertex_attributes,
            convert_coordinates,
            meshes_on_skinned_nodes,
            meshes_on_non_skinned_nodes,
        ) {
            *user_mesh = Some(mesh);
        }
    }
}

pub struct GltfDracoDecoderPlugin;

impl Plugin for GltfDracoDecoderPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(target_family = "wasm")]
        bevy::tasks::block_on(async {
            app.world_mut()
                .resource_mut::<GltfExtensionHandlers>()
                .0
                .write()
                .await
                .push(Box::new(GltfDracoDecoderExtensionHandler::default()))
        });
        #[cfg(not(target_family = "wasm"))]
        app.world_mut()
            .resource_mut::<GltfExtensionHandlers>()
            .0
            .write_blocking()
            .push(Box::new(GltfDracoDecoderExtensionHandler::default()));
    }
}
