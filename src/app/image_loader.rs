use crate::app::*;
use crate::db::FullArticle;
use std::sync::mpsc::Receiver;

pub(super) fn load_images(
    app: &mut App,
    full: &FullArticle,
) -> (Vec<ImageLoadState>, Option<Receiver<ImageLoadMsg>>) {
    let mut images: Vec<ImageLoadState> = Vec::new();
    let mut to_fetch: Vec<(usize, u32, String)> = Vec::new();

    for (i, img) in full.images.iter().enumerate() {
        if let Some(ref data) = img.data {
            match image::ImageReader::new(std::io::Cursor::new(data)).with_guessed_format() {
                Ok(reader) => match reader.decode() {
                    Ok(dyn_img) => {
                        let protocol = app.picker.new_resize_protocol(dyn_img);
                        images.push(ImageLoadState::Loaded(Box::new(ImageState {
                            protocol,
                            alt_text: img.alt_text.clone(),
                        })));
                    }
                    Err(_) => {
                        images.push(ImageLoadState::Failed);
                    }
                },
                Err(_) => {
                    images.push(ImageLoadState::Failed);
                }
            }
        } else {
            images.push(ImageLoadState::Loading);
            to_fetch.push((i, img.id, img.url.clone()));
        }
    }

    let rx = if to_fetch.is_empty() {
        None
    } else {
        let (tx, rx) = std::sync::mpsc::channel();
        let db = app.db.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            for (idx, image_id, url) in to_fetch {
                let result = rt.block_on(crate::sync::fetch_and_store_image(&db, image_id, &url));
                match result {
                    Ok(()) => {
                        if let Ok(Some(img)) = db.get_image_data(image_id) {
                            let _ = tx.send(ImageLoadMsg::Loaded(idx, img));
                        } else {
                            let _ = tx.send(ImageLoadMsg::Failed(idx));
                        }
                    }
                    Err(_) => {
                        let _ = tx.send(ImageLoadMsg::Failed(idx));
                    }
                }
            }
        });
        Some(rx)
    };

    (images, rx)
}

pub(super) fn poll_image_load(app: &mut App) {
    if let Some(ref mut reader) = app.reader
        && let Some(rx) = reader.image_load_rx.take()
    {
        loop {
            match rx.try_recv() {
                Ok(ImageLoadMsg::Loaded(idx, data)) => {
                    if idx < reader.images.len() {
                        match image::ImageReader::new(std::io::Cursor::new(&data))
                            .with_guessed_format()
                        {
                            Ok(rdr) => {
                                if let Ok(dyn_img) = rdr.decode() {
                                    let protocol = app.picker.new_resize_protocol(dyn_img);
                                    let alt_text = reader
                                        .article
                                        .images
                                        .get(idx)
                                        .and_then(|img| img.alt_text.clone());
                                    reader.images[idx] =
                                        ImageLoadState::Loaded(Box::new(ImageState {
                                            protocol,
                                            alt_text,
                                        }));
                                } else {
                                    reader.images[idx] = ImageLoadState::Failed;
                                }
                            }
                            Err(_) => {
                                reader.images[idx] = ImageLoadState::Failed;
                            }
                        }
                    }
                }
                Ok(ImageLoadMsg::Failed(idx)) => {
                    if idx < reader.images.len() {
                        reader.images[idx] = ImageLoadState::Failed;
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    reader.image_load_rx = Some(rx);
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    break;
                }
            }
        }
    }
}
