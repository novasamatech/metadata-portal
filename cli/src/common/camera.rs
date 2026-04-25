use std::path::Path;

use anyhow::{anyhow, bail};
use image::GrayImage;
use indicatif::ProgressBar;
use opencv::{
    imgproc::{cvt_color, COLOR_BGR2GRAY},
    prelude::*,
    videoio,
};
use qr_reader_phone::process_payload::{process_decoded_payload, InProgress, Ready};

pub(crate) fn read_qr_file(source_file: &Path) -> anyhow::Result<String> {
    let mut camera = create_camera(source_file)?;

    let mut out = Ready::NotYet(InProgress::None);
    let mut line = String::new();

    let pb = ProgressBar::new(1);
    loop {
        match out {
            Ready::NotYet(decoding) => {
                if let InProgress::Fountain(f) = &decoding {
                    pb.set_length(f.total as u64);
                    pb.set_position(f.collected() as u64)
                }
                out = match camera_capture(&mut camera) {
                    Ok(img) => process_qr_image(&img, decoding)?,
                    Err(_) => Ready::NotYet(decoding),
                };
            }
            Ready::Yes(a) => {
                pb.finish_and_clear();
                line.push_str(&hex::encode(a));
                break;
            }
            Ready::BananaSplitPasswordRequest => {
                pb.finish_and_clear();
                bail!("Banana split is not supported.");
            }
            Ready::BananaSplitReady(_) => {
                pb.finish_and_clear();
                bail!("Banana split is not supported.");
            }
        }
    }
    Ok(line)
}

fn create_camera(source_file: &Path) -> anyhow::Result<videoio::VideoCapture> {
    #[cfg(not(ocvrs_opencv_branch_32))]
    Ok(videoio::VideoCapture::from_file(
        source_file.to_str().unwrap(),
        videoio::CAP_ANY,
    )?)
}

fn camera_capture(camera: &mut videoio::VideoCapture) -> anyhow::Result<GrayImage> {
    let mut frame = Mat::default();
    match camera.read(&mut frame) {
        Ok(_) if frame.size()?.width > 0 => (),
        Ok(_) => bail!("Zero frame size."),
        Err(e) => bail!("Can`t read camera. {}", e),
    };

    let mut gray = Mat::default();
    cvt_color(&frame, &mut gray, COLOR_BGR2GRAY, 0)?;

    let rows = gray.rows() as u32;
    let cols = gray.cols() as u32;

    if gray.is_continuous() {
        let bytes = gray.data_bytes()?;
        GrayImage::from_raw(cols, rows, bytes.to_vec())
            .ok_or_else(|| anyhow!("failed to build GrayImage from continuous Mat"))
    } else {
        let mut buf = Vec::with_capacity((rows * cols) as usize);
        for y in 0..rows as i32 {
            let row: &[u8] = gray.at_row::<u8>(y)?;
            buf.extend_from_slice(&row[..cols as usize]);
        }
        GrayImage::from_raw(cols, rows, buf)
            .ok_or_else(|| anyhow!("failed to build GrayImage from non-continuous Mat"))
    }
}

fn process_qr_image(image: &GrayImage, decoding: InProgress) -> anyhow::Result<Ready> {
    let mut qr_decoder = quircs::Quirc::new();
    let codes = qr_decoder.identify(image.width() as usize, image.height() as usize, image);

    match codes.last() {
        Some(Ok(code)) => match code.decode() {
            Ok(decoded) => {
                process_decoded_payload(decoded.payload, &None, decoding).map_err(|e| e.into())
            }
            Err(_) => Ok(Ready::NotYet(decoding)),
        },
        Some(_) => Ok(Ready::NotYet(decoding)),
        None => Ok(Ready::NotYet(decoding)),
    }
}
