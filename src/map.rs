#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::*;

#[cfg(target_arch = "wasm32")]
use web_sys::Blob;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(raw_module = "./wasm-utils/utils.js")]
extern "C" {
    fn read_json(path: &str) -> JsValue;
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(raw_module = "./wasm-utils/utils.js")]
extern "C" {
    fn read_image(path: &str) -> JsValue;
}

#[derive(Debug)]
pub struct TileSet {
    pub image: Vec<u8>,
    pub columns: u32,
    pub tile_count: u32,
    pub image_size: (u32, u32),
}

#[derive(Debug)]
pub struct Object {
    pub name: String,
    pub id: u32,
    pub position: (u32, u32),
    pub size: (u32, u32),
}

#[derive(Debug)]
pub struct TileLayer {
    pub data: Vec<u32>,
    pub id: u32,
    pub name: String,
    pub size: (u32, u32),
}

#[derive(Debug)]
pub enum Layers {
    ObjectGroup {
        id: u32,
        name: String,
        visible: bool,
        objects: Vec<Object>,
    },
    TileLayer {
        id: u32,
        name: String,
        visible: bool,
        data: Vec<i32>,
    },
}

#[derive(Debug)]
pub struct Map {
    pub size: (u32, u32),
    pub tile_size: (u32, u32),
    pub layers: Vec<Layers>,
    pub tileset: TileSet,
}

pub async fn load_map(path_data: &str) -> Map {
    let json_file = {
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                let promise_as_jsvalue = read_json(path_data);
                let promise = js_sys::Promise::from(promise_as_jsvalue);
                let future = JsFuture::from(promise);
                let result: Result<JsValue, JsValue> = future.await;
                let jsvalue = result.clone().unwrap();
                serde_wasm_bindgen::from_value::<serde_json::Value>(jsvalue).unwrap()
            } else {
                let data  = std::fs::read_to_string(path_data).unwrap();
                serde_json::from_str::<serde_json::Value>(data.as_str()).unwrap()
            }
        }
    };

    let filename = {
        let mut f = "./resources/".to_owned();
        f.push_str(json_file["tilesets"][0]["image"].as_str().unwrap());
        f
    };

    let spritesheet = {
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                let promise_as_jsvalue = read_image(filename.as_str());
                let promise = js_sys::Promise::from(promise_as_jsvalue);
                let future = JsFuture::from(promise);
                let result: Result<JsValue, JsValue> = future.await;
                let jsvalue = result.unwrap();
                let blob: Blob = jsvalue.into();
                let array_buffer_promise: JsFuture = blob.array_buffer().into();
                let array_buffer: JsValue = array_buffer_promise.await.unwrap();
                js_sys::Uint8Array::new(&array_buffer).to_vec()
            } else {
                std::fs::read(filename).unwrap()
            }
        }
    };

    let map = Map {
        size: (
            json_file["width"].as_u64().unwrap() as u32,
            json_file["height"].as_u64().unwrap() as u32,
        ),
        tile_size: (
            json_file["tilewidth"].as_u64().unwrap() as u32,
            json_file["tileheight"].as_u64().unwrap() as u32,
        ),
        layers: {
            let mut layers: Vec<Layers> = vec![];
            for value in json_file["layers"].as_array().unwrap() {
                match value["type"].as_str().unwrap() {
                    "tilelayer" => {
                        layers.push(Layers::TileLayer {
                            id: value["id"].as_u64().unwrap() as u32,
                            name: value["name"].as_str().unwrap().to_string(),
                            visible: value["visible"].as_bool().unwrap(),
                            data: value["data"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|x| x.as_i64().unwrap() as i32)
                                .collect::<Vec<i32>>(),
                        });
                    }
                    "objectgroup" => {
                        // println!("OBJECT GROUP")
                    }
                    _ => {}
                }
            }
            layers
        },
        tileset: TileSet {
            image: spritesheet,
            columns: json_file["tilesets"][0]["columns"].as_u64().unwrap() as u32,
            tile_count: json_file["tilesets"][0]["tilecount"].as_u64().unwrap() as u32,
            image_size: (
                json_file["tilesets"][0]["tilewidth"].as_u64().unwrap() as u32,
                json_file["tilesets"][0]["tileheight"].as_u64().unwrap() as u32,
            ),
        },
    };

    map
}
