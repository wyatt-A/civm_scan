use eframe::egui;
use ndarray::{Array2, Order};
use ndarray_stats::QuantileExt;
use eframe::egui::ColorImage;


pub fn array_to_image(scalar_image:&Array2<f32>) -> ColorImage {

    let mut image = scalar_image.clone();

    let s = scalar_image.shape();
    let n = scalar_image.len();

    // normalize image
    let min = scalar_image.min().expect("cannot determine min");
    image -= *min;
    let max = image.max().expect("cannot determine max");
    image /= *max;

    // convert to color image that egui can use
    let gray_pixels = image.to_shape((n,Order::RowMajor)).expect("shape don't fit").to_vec();
    let mut color_pixels = Vec::<u8>::with_capacity(4*n);
    for x in gray_pixels {
        color_pixels.extend(egui::Color32::from_gray((x*255.0) as u8).to_srgba_unmultiplied())
    }
    ColorImage::from_rgba_unmultiplied([s[1],s[0]],color_pixels.as_slice())

}
