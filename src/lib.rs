use bevy_app::{App, Plugin};
use bevy_asset::LoadContext;
#[cfg(not(target_family = "wasm"))]
use bevy_gltf::extensions::GltfExtensionHandlers;
use bevy_gltf::{
    extensions::GltfExtensionHandler,
    gltf::{Document, Gltf as JsonGltf, Primitive},
};
#[cfg(target_arch = "wasm32")]
use draco_decoder::decode_mesh_with_config;
#[cfg(target_arch = "wasm32")]
use futures::channel::oneshot;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

use crate::khr_draco_mesh_compression::DracoExtension;

mod khr_draco_mesh_compression;

#[derive(Default, Clone)]
struct GltfDracoDecoderExtensionHandler;

#[async_trait::async_trait]
impl GltfExtensionHandler for GltfDracoDecoderExtensionHandler {
    fn dyn_clone(&self) -> Box<dyn GltfExtensionHandler> {
        Box::new((*self).clone())
    }

    #[cfg(not(target_arch = "wasm32"))]
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

    #[cfg(target_arch = "wasm32")]
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
        {
            if let Some(encoded_data) = draco_ext.get_encoded_data(gltf_json, &buffer_data) {
                let data = encoded_data.to_vec();
                let (tx, rx) = oneshot::channel();

                spawn_local(async move {
                    let result = decode_mesh_with_config(&data).await;
                    let _ = tx.send(result);
                });

                // Wait for result - rx.await IS Send because oneshot::Receiver<T> is Send when T: Send
                match rx.await {
                    Ok(Some(result)) => {
                        *out_data = Some(vec![result.data]);
                        *out_doc = draco_ext.build_document(&gltf_primitive, &result.config);
                    }
                    Ok(None) => {
                        tracing::warn!("Draco decode returned no result");
                    }
                    Err(_) => {
                        tracing::warn!("Draco decode channel closed unexpectedly");
                    }
                }
            }
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
