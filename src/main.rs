use std::{env::temp_dir, fs::File, io::Write, str::FromStr, time::Duration};

use anyhow::anyhow;
use rand::prelude::IndexedRandom;
use reqwest::IntoUrl;
use tokio::time::sleep;
use url::Url;

use crate::apache_files_scraper::apache_grzyby_index_iter;
mod apache_files_scraper;

async fn reqwest_get_text<T: IntoUrl>(url: T) -> anyhow::Result<String> {
    // reqwest::get(url).await?.text().await.map_err(|a| a.into())
    let mut res = reqwest::get(url).await?;

    const MAX_BUF: usize = 40962;
    let mut buf: Vec<u8> = Vec::with_capacity(MAX_BUF);

    while let Some(chunk) = res.chunk().await? {
        buf.extend(chunk.iter().take(MAX_BUF - buf.len()));
        if buf.len() >= MAX_BUF {
            break;
        }
    }

    Ok(String::from_utf8_lossy(&buf).into_owned())
}

async fn get_random_image_folder() -> anyhow::Result<Url> {
    let url = Url::from_str("https://www.grzyby.pl/foto/").expect("valid url");
    let html = reqwest_get_text(url.join("?C=M;O=D")?).await?;
    let page = scraper::Html::parse_document(&html);
    let folders =
        apache_grzyby_index_iter(&page).ok_or(anyhow!("Can't parse grzyby.pl directory"))?;

    let folder = folders
        .filter_map(|x| x.get_link().and_then(|a| x.get_date().map(|b| (a, b))))
        // .filter(|x| {
        //     x.text()
        //         .next()
        //         .is_some_and(|x| x.len() == 3 && x.chars().all(|x| x.is_ascii_digit() || x == '/'))
        // })
        .filter(|x| x.0.text().next().is_some_and(|x| !x.contains('_') && !x.contains('-')))
        .filter_map(|x| x.0.attr("href").map(|a| (a, x.1)))
        .next()
        // .max_by_key(|x| x.1)
        .ok_or(anyhow!("Cannot get the newest folder"))?;

    // let Some(folder) = folders.choose(&mut rand::rng()) else {
    //     return Err(anyhow!("No image grzyby.pl folder"));
    // };

    url.join(folder.0)
        .map_err(|_| anyhow!("Invaild grzyby.pl folder"))
}

async fn get_random_image(folder_url: Url) -> anyhow::Result<Url> {
    let html = reqwest_get_text(folder_url.join("?C=M;O=D")?).await?;
    let page = scraper::Html::parse_document(&html);
    let folders =
        apache_grzyby_index_iter(&page).ok_or(anyhow!("Can't parse grzyby.pl image directory",))?;

    let mut images = folders
        // .filter(|x| x.get_size().is_some_and(|x| x < 1 * 1024u64.pow(2)))
        .filter_map(|x| x.get_link())
        .filter_map(|x| x.attr("href"))
        .filter(|x| x.ends_with(".jpg") || x.ends_with(".jpeg") || x.ends_with(".png"))
        .filter(|x| !x.contains("is.") && !x.contains("icon."))
        // .take(40)
        .collect::<Vec<_>>();
    images.pop();

    // dbg!(&images);

    let Some(image) = images.choose(&mut rand::rng()) else {
        return Err(anyhow!("No images inside grzyby.pl folder"));
    };

    folder_url
        .join(image)
        .map_err(|_| anyhow!("Invaild grzyby.pl image"))
}

async fn set_grzyb_wallpaper() -> anyhow::Result<()> {
    let folder_url = get_random_image_folder().await?;
    let image_url = get_random_image(folder_url).await?;

    let image_name = image_url
        .as_str()
        .rsplit_once("/")
        .map(|x| x.1)
        .unwrap_or(image_url.as_str())
        .to_owned();

    let bytes = reqwest::get(image_url).await?.bytes().await?;

    let path = temp_dir().join(image_name);
    let mut file = File::create_new(path.clone())?;
    file.write_all(&bytes)?;
    wallpaper::set_from_path(path.to_str().ok_or(anyhow!("Non-unicode characters."))?)
        .map_err(|e| anyhow!(e.to_string()))?;
    // wallpaper::set_mode(wallpaper::Mode::Span).unwrap();
    // print!("{}", path.display());

    Ok(())
}

#[tokio::main]
async fn main() {
    let mut cooldown = Duration::from_secs(1);
    loop {
        match set_grzyb_wallpaper().await {
            Ok(_) => return,
            Err(e) => {
                eprintln!("{e}");
                if cooldown > Duration::from_secs(3600) {
                    return;
                }
                sleep(cooldown).await;
                cooldown *= 2;
            }
        }
    }
}
