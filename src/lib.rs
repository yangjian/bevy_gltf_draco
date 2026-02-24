use bevy_app::{App, Plugin};
use bevy_asset::LoadContext;
#[cfg(not(target_family = "wasm"))]
use bevy_gltf::extensions::GltfExtensionHandlers;
use bevy_gltf::{
    extensions::GltfExtensionHandler,
    gltf::{Document, Gltf as JsonGltf, Primitive},
};

use crate::khr_draco_mesh_compression::DracoExtension;

mod khr_draco_mesh_compression;

#[derive(Default, Clone)]
struct GltfDracoDecoderExtensionHandler;

#[async_trait::async_trait]
impl GltfExtensionHandler for GltfDracoDecoderExtensionHandler {
    fn dyn_clone(&self) -> Box<dyn GltfExtensionHandler> {
        Box::new((*self).clone())
    }

    async fn on_gltf_primitive(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf_json: &JsonGltf,
        gltf_primitive: &Primitive,
        buffer_data: &[Vec<u8>],
        out_doc: &mut Option<Document>,
        out_data: &mut Option<Vec<Vec<u8>>>,
    ) {
        if let Some(draco_ext) =
            DracoExtension::parse(load_context, &gltf_json, gltf_primitive).as_mut()
            && let Some((config, decode_data)) =
                draco_ext.decode_mesh(gltf_json, &buffer_data).await
        {
            *out_data = Some(decode_data);
            *out_doc = draco_ext.build_document(&gltf_primitive, &config);
        }
    }
}

pub struct GltfDracoDecoderPlugin;

impl Plugin for GltfDracoDecoderPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(target_family = "wasm")]
        bevy_tasks::block_on(async {
            use bevy_gltf::extensions::GltfExtensionHandlers;

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
