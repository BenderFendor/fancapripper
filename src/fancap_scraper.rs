use scraper::{Html, Selector};

#[derive(Debug, PartialEq, Clone)]
pub struct FancapImage {
    pub url: String,
    pub thumb_url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_image_urls() {
        let html = r#"
            <html>
                <body>
                    <img src="https://moviethumbs.fancaps.net/1234.jpg" />
                    <img src="https://other.site.com/image.jpg" />
                    <img src="https://moviethumbs.fancaps.net/5678.jpg" />
                </body>
            </html>
        "#;

        let images = extract_image_urls(html);

        assert_eq!(images.len(), 2);
        assert_eq!(
            images[0].thumb_url,
            "https://moviethumbs.fancaps.net/1234.jpg"
        );
        assert_eq!(
            images[0].url,
            "https://cdni.fancaps.net/file/fancaps-movieimages/1234.jpg"
        );
        assert_eq!(
            images[1].thumb_url,
            "https://moviethumbs.fancaps.net/5678.jpg"
        );
        assert_eq!(
            images[1].url,
            "https://cdni.fancaps.net/file/fancaps-movieimages/5678.jpg"
        );
    }
}

pub fn extract_image_urls(html_content: &str) -> Vec<FancapImage> {
    let mut image_urls: Vec<FancapImage> = vec![];
    let document = Html::parse_document(html_content);
    let image_select = Selector::parse("img").unwrap();

    for element in document.select(&image_select) {
        if let Some(src) = element.value().attr("src") {
            // Handle old format: moviethumbs.fancaps.net
            if src.contains("moviethumbs") {
                let replaced_src = src.replace(
                    "https://moviethumbs.fancaps.net",
                    "https://cdni.fancaps.net/file/fancaps-movieimages",
                );
                image_urls.push(FancapImage {
                    url: replaced_src,
                    thumb_url: src.to_string(),
                });
            }
            // Handle new format: mvt.fancaps.net (e.g., https://mvt.fancaps.net/554335.jpg)
            else if src.contains("mvt.fancaps.net") {
                // Extract the image ID from the URL and construct the full-size URL
                // mvt.fancaps.net/554335.jpg -> cdni.fancaps.net/file/fancaps-movieimages/554335.jpg
                let replaced_src = src.replace(
                    "https://mvt.fancaps.net",
                    "https://cdni.fancaps.net/file/fancaps-movieimages",
                );
                image_urls.push(FancapImage {
                    url: replaced_src,
                    thumb_url: src.to_string(),
                });
            }
        }
    }

    image_urls
}
