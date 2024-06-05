use reqwest::{header::USER_AGENT, Client, Error};
use tokio;
use scraper::{error, Html, Selector};
use futures::{stream, StreamExt};
use tokio::fs::File;
use std::fs;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashSet;
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() {
    let url: &str = "https://fancaps.net/movies/MovieImages.php?name=Howl_s_Moving_Castle&movieid=220";
    let max_page: i32 = 500; // This should just give the max_page by guess it is less then 500
    let header: &str = "Mozilla/5.0 (Windows NT 10.0; Win64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/89.0.4389.82 Safari/537.36";

    get_max_page(url, max_page,header).await.unwrap();
}

async fn get_max_page(url: &str, initial_max_page: i32,header: &str) -> Result<i32,Error> {
    let client = reqwest::Client::new();
    let mut max_page = initial_max_page;

    let page_url = format!("{}&page={}", url, max_page);
    println!("Page url is {}", page_url);

    let response = client
        .get(&page_url)
        .header(USER_AGENT,header)
        .send()
        .await;

    match response {
        Ok(res) => {
            let html_content = res.text().await.unwrap();
            let document = Html::parse_document(&html_content);
            let pagination_select = Selector::parse("ul.pagination li:not(:first-child):not(:last-child) a").map_err(|_| "Invalid selector");

            let elements: Vec<_> = document.select(&pagination_select.unwrap()).collect();
            if let Some(last_element) = elements.last() {
                if let Ok(last_page_number) = last_element.text().collect::<String>().trim().parse::<i32>() {
                    if last_page_number > max_page {
                        max_page = last_page_number;
                    } else {
                        println!("Done! Max Page Number is {}", last_page_number);
                        rippage(url, last_page_number, header).await.unwrap();
                        return Ok(last_page_number);
                    }
                }
            }
            return Ok(max_page);
        }
        Err(err) => return Err(err),
    }
}

async fn rippage(url: &str, max_page: i32,header: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut page_urls: Vec<String> = vec![];
    for number in (0..=max_page).rev() {
        let page_url = format!("{}&page={}", url, number);
        page_urls.push(page_url);
    }

    let mut image_urls: HashSet<String> = HashSet::new();
    let client = reqwest::Client::new();

    println!("Ripping Pages");

    for page_url in page_urls {
        let response = client
            .get(&page_url)
            .header(USER_AGENT,header)
            .send()
            .await;

        match response {
            Ok(res) => {
                let html_content = res.text().await?;
                let images = get_images(&html_content);
                image_urls.extend(images);
            }
            Err(err) => {
                eprintln!("Request error: {:?}", err);
            }
        }
    }

    let url_parts: Vec<&str> = url.split('?').collect();
    if url_parts.len() > 1 {
        let query_params: Vec<&str> = url_parts[1].split('&').collect();
        for param in query_params {
            let key_value: Vec<&str> = param.split('=').collect();
            if key_value.len() > 1 && key_value[0] == "name" {
                let folder_name = key_value[1].replace("_", " ");
                let path = format!("fancaps/{}", folder_name);
                fs::create_dir_all(&path)?;
                download_images(image_urls, path).await?;
                break;
            }
        }
    }

    Ok(())
}

async fn download_images(image_urls: HashSet<String>, folder_name: String) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let pb = ProgressBar::new(image_urls.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {human_pos}/{human_len} ({decimal_bytes_per_sec}, ETA: {eta})")?
            .progress_chars("#>-"),
    );

    // Create a stream from the image URLs
    let stream = stream::iter(image_urls.into_iter().map(|url| {
        let client = client.clone();
        let folder_name = folder_name.clone();
        let pb = pb.clone();

        // Spawn a new task for each download
        tokio::spawn(async move {
            let response = client.get(&url).send().await?;
            let bytes = response.bytes().await?;
            let file_name = url.split('/').last().unwrap();
            let file_path = format!("{}/{}", &folder_name, file_name);
            let mut file = File::create(&file_path).await?;
            file.write_all(&bytes).await?;
            pb.inc(1);
            Result::<_, Box<dyn std::error::Error + Send + Sync>>::Ok(())
        })
    }));

    // Buffer up to 10 downloads to run concurrently
    stream.buffer_unordered(500).collect::<Vec<_>>().await;

    pb.finish_with_message("Download completed");
    Ok(())
}

fn get_images(html_content: &str) -> Vec<String> {
    let mut image_urls: Vec<String> = vec![];
    let document = Html::parse_document(html_content);
    let image_select = Selector::parse("img").unwrap();

    for element in document.select(&image_select) {
        if let Some(src) = element.value().attr("src") {
            if src.contains("moviethumbs") {
                let replaced_src = src.replace("https://moviethumbs.fancaps.net", "https://cdni.fancaps.net/file/fancaps-movieimages");
                image_urls.push(replaced_src);
            }
        }
    }

    image_urls
}
