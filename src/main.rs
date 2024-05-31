use reqwest::header::USER_AGENT;
use tokio;
use scraper::{Element, Html, Selector};

#[tokio::main]
async fn main() {
    let url: &str = "https://fancaps.net/movies/MovieImages.php?name=Howl_s_Moving_Castle&movieid=220";
    let maxpage: i32 = 5;
    let maxnumber = match getpages(url, maxpage).await {
        Ok(number) => number,
        Err(_) => {
            // Handle the error case here, if needed
            return;
        }
    };
}
async fn getpages(url: &str, maxpage: i32) -> Result<i32, &'static str> {
    Box::pin(async move {
      let client = reqwest::Client::new();

    let mut max_page = maxpage;


    let page_url = format!("{}&page={}", url, max_page);

    println!("Page url is {}",page_url);

    let response = client
        .get(&page_url)
        .header(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/89.0.4389.82 Safari/537.36")
        .send()
        .await;

    match response {
        Ok(res) => {
            let html_content = res.text().await.unwrap();

            let document = Html::parse_document(&html_content);
            let pagination_select = Selector::parse("ul.pagination li:not(:first-child):not(:last-child) a").unwrap();
            
            let elements: Vec<_> = document.select(&pagination_select).collect();
            let elementclone = elements.clone();
            'outer: for element in elements {
                let page_text = element.text().collect::<String>().trim().to_string();
                match page_text.parse::<i32>() {
                    Ok(page_number) => {
                        if page_number > max_page {
                         
                 
                            max_page = page_number;
                            println!("The max page is {}", max_page);
                            if let Err(e) = getpages(url, max_page).await {
                                if e == "Done" {
                                    // If the error is "Done", return it immediately
                                    return Err("Done");
                                }
                            }
                            getpages(url, max_page).await;
                        }
                        if let Some(last_page_number) = elementclone.last().map(|e| e.text().collect::<String>().trim().to_string()) {
                            if let Ok(last_page_number) = last_page_number.parse::<i32>() {
                                if last_page_number <= max_page {
                                    println!("Done!");
                                    rippage(url, max_page).await;
                                    println!("{:?}", max_page);
                                    return Err("Done");
                                }
                            }
                        }
                    },
                    Err(_) => {
                        eprintln!("Failed to parse page number: '{}'", page_text);
                    }
                }
            }
        },
        Err(err) => {
            eprintln!("Request error: {:?}", err);
        }    
    }
    Ok(max_page)
    }).await
}


async fn rippage(url: &str,maxnumber : i32)
{ 
    let mut pageurls: Vec<String> = vec![];
    let count = 0;
    for number in (0..=maxnumber).rev() {
        let page_url = format!("{}&page={}", url, number);
        pageurls.push(page_url);
    }

    let mut imageurls: Vec<String> = vec![];

    for pages in pageurls
    {
        let client = reqwest::Client::new();
    
        let response = client
            .get(pages)
            .header(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/89.0.4389.82 Safari/537.36")
            .send()
            .await;
    
        let images: Vec<String>;
    
        match response {
            Ok(res) => {
                let html_content = res.text().await.unwrap();
                //println!("{}", html_content);

                images = getimages(html_content);
               // println!("{:#?}",images);
                imageurls.extend(images);
            },
            Err(err) => {
                eprintln!("Request error: {:?}", err);
            }    
        }   
    }

    println!("{:#?}",imageurls)

}
fn getimages(html_content: String) -> Vec<String>
{
    let mut imageurls: Vec<String> = vec![];

    let document = Html::parse_document(&html_content);
    let imageselect = Selector::parse("img").unwrap();

    for element in document.select(&imageselect) {
        let text = element.value().attr("src").unwrap();
        if text.contains("moviethumbs") {
            let replacetext = text.replace( "https://moviethumbs.fancaps.net","https://cdni.fancaps.net/file/fancaps-movieimages");
         
           // println!("{}", replacetext);

            imageurls.push(replacetext);
        }

        // https://cdni.fancaps.net/file/fancaps-movieimages/554335.jpg
        // https://moviethumbs.fancaps.net/554335.jpg
    }

    imageurls
}
