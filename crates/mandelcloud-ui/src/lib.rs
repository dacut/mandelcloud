use {
    js_sys::{JsString, Reflect},
    std::error::Error,
    wasm_bindgen::{prelude::*, Clamped, JsValue},
    web_sys::{window, CanvasRenderingContext2d, HtmlCanvasElement, ImageData},
    web_time::Instant,
};

type BoxError = Box<dyn Error + Send + Sync + 'static>;

#[derive(Debug)]
pub enum RenderingEngine {
    Local,
    Remote,
}

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);

    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn _log_str(s: &str);
}

macro_rules! console_log {
    // Note that this is using the `log` function imported above during
    // `bare_bones`
    ($($t:tt)*) => (_log_str(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
pub fn start() {
    console_log!("Loading Rust WASM module");

    let start = Instant::now();
    let m = get_canvas().unwrap();
    let context = m.get_context("2d").unwrap().unwrap().dyn_into::<CanvasRenderingContext2d>().unwrap();

    let width = m.width();
    let height = m.height();
    let mut pixels = vec![0; (width * height * 4) as usize];

    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) as usize * 4;
            pixels[i] = (x * 255 / width) as u8;
            pixels[i + 1] = 0;
            pixels[i + 2] = (y * 255 / height) as u8;
            pixels[i + 3] = 255;
        }
    }

    let image = ImageData::new_with_u8_clamped_array_and_sh(Clamped(&pixels), width, height).unwrap();
    context.put_image_data(&image, 0.0, 0.0).unwrap();
    let elapsed = start.elapsed();
    console_log!("Rendered in {} ms", elapsed.as_millis());
}

#[wasm_bindgen]
pub fn render_mb_canvas() {
    console_log!("Rendering Mandelbrot canvas");
    let engine = get_rendering_engine().unwrap();
    match engine {
        RenderingEngine::Local => render_mb_canvas_local(),
        _ => todo!("Remote rendering engine not implemented"),
    }
}

fn render_mb_canvas_local() {
    
}

fn get_rendering_engine() -> Result<RenderingEngine, BoxError> {
    let Some(w) = window() else {
        return Err("No window found".to_string().into());
    };

    let Some(doc) = w.document() else {
        return Err("No document found".to_string().into());
    };

    for el in doc.get_elements_by_name("rendering-engine").values() {
        match el {
            Ok(el) => match Reflect::get(&el, &JsValue::from_str("checked")) {
                Ok(checked) if checked.is_truthy() => match Reflect::get(&el, &JsValue::from_str("value")) {
                    Ok(value) => {
                        let Some(value) = value.dyn_ref::<JsString>() else {
                            return Err("Rendering engine value is not a string".to_string().into());
                        };

                        if value == "local" {
                            return Ok(RenderingEngine::Local);
                        } else if value == "remote" {
                            return Ok(RenderingEngine::Remote);
                        } else {
                            return Err("Invalid rendering engine value".to_string().into());
                        }
                    }
                    Err(e) => return Err(format!("Failed to get rendering engine value: {:?}", e).into()),
                },
                _ => (),
            },
            Err(e) => return Err(format!("Failed to get rendering engine checked state: {:?}", e).into()),
        }
    }

    Err("No rendering engine selected".to_string().into())
}

fn get_canvas() -> Result<HtmlCanvasElement, BoxError> {
    let Some(w) = window() else {
        return Err("No window found".to_string().into());
    };

    let Some(doc) = w.document() else {
        return Err("No document found".to_string().into());
    };

    let Some(m) = doc.get_element_by_id("m") else {
        return Err("Mandelbrot canvas frame not found".to_string().into());
    };

    m.dyn_into::<HtmlCanvasElement>().map_err(|_| "Mandelbrot canvas frame is not a canvas element".to_string().into())
}
