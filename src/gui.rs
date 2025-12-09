use crate::fancap_scraper::{extract_image_urls, FancapImage};
use iced::widget::{
    button, column, container, image, mouse_area, opaque, row, scrollable, stack, text, text_input,
    Column, Row,
};
use iced::{Background, Border, Color, Element, Length, Task, Theme};

// Design Direction: Retro-Futuristic / Terminal-inspired
// Color palette: Deep charcoal, electric cyan accent, muted grays

const BG_DARK: Color = Color::from_rgb(0.08, 0.08, 0.10);
const BG_SURFACE: Color = Color::from_rgb(0.12, 0.12, 0.14);
const BG_ELEVATED: Color = Color::from_rgb(0.16, 0.16, 0.18);
const ACCENT: Color = Color::from_rgb(0.0, 0.78, 0.82);
const TEXT_PRIMARY: Color = Color::from_rgb(0.92, 0.92, 0.94);
const TEXT_MUTED: Color = Color::from_rgb(0.55, 0.55, 0.58);
const BORDER_COLOR: Color = Color::from_rgb(0.25, 0.25, 0.28);
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/89.0.4389.82 Safari/537.36";

#[derive(Debug, Clone)]
pub enum Message {
    UrlChanged(String),
    SubmitUrl,
    UrlsFound(Result<Vec<FancapImage>, String>),
    ImageLoaded(usize, Result<Vec<u8>, String>),
    OpenModal(usize),
    CloseModal,
    FullImageLoaded(Result<Vec<u8>, String>),
    NextPage,
    PrevPage,
}

#[derive(Default)]
pub struct FancapRipper {
    pub url_input: String,
    pub base_url: String,
    pub current_page: usize,
    pub state: AppState,
    pub modal: Option<ModalState>,
}

#[derive(Debug)]
pub struct ModalState {
    pub image_url: String,
    pub thumb_url: String,
    pub image_handle: Option<image::Handle>,
    pub loading: bool,
}

#[derive(Debug, PartialEq)]
pub enum AppState {
    Idle,
    LoadingUrls,
    DisplayingImages(Vec<ImageCard>),
    Error(String),
}

#[derive(Debug, PartialEq)]
pub struct ImageCard {
    pub url: String,
    pub thumb_url: String,
    pub state: ImageState,
}

#[derive(Debug, PartialEq)]
pub enum ImageState {
    Loading,
    Loaded(image::Handle),
    Error,
}

impl Default for AppState {
    fn default() -> Self {
        Self::Idle
    }
}

impl FancapRipper {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::UrlChanged(url) => {
                self.url_input = url;
                Task::none()
            }
            Message::SubmitUrl => {
                let url = self.url_input.clone();
                if url.is_empty() {
                    return Task::none();
                }

                // Store base URL and reset page
                self.base_url = url.split("&page=").next().unwrap_or(&url).to_string();
                self.current_page = 1;
                self.state = AppState::LoadingUrls;

                let fetch_url = format!("{}&page={}", self.base_url, self.current_page);
                Task::perform(fetch_urls(fetch_url), Message::UrlsFound)
            }
            Message::NextPage => {
                if self.base_url.is_empty() {
                    return Task::none();
                }
                self.current_page += 1;
                self.state = AppState::LoadingUrls;
                let fetch_url = format!("{}&page={}", self.base_url, self.current_page);
                Task::perform(fetch_urls(fetch_url), Message::UrlsFound)
            }
            Message::PrevPage => {
                if self.base_url.is_empty() || self.current_page <= 1 {
                    return Task::none();
                }
                self.current_page -= 1;
                self.state = AppState::LoadingUrls;
                let fetch_url = format!("{}&page={}", self.base_url, self.current_page);
                Task::perform(fetch_urls(fetch_url), Message::UrlsFound)
            }
            Message::UrlsFound(Ok(images)) => {
                let cards: Vec<ImageCard> = images
                    .into_iter()
                    .map(|img| ImageCard {
                        url: img.url,
                        thumb_url: img.thumb_url,
                        state: ImageState::Loading,
                    })
                    .collect();

                let mut tasks = Vec::new();
                for (i, card) in cards.iter().enumerate() {
                    tasks.push(Task::perform(
                        fetch_image(card.thumb_url.clone()),
                        move |res| Message::ImageLoaded(i, res),
                    ));
                }

                self.state = AppState::DisplayingImages(cards);
                Task::batch(tasks)
            }
            Message::UrlsFound(Err(e)) => {
                self.state = AppState::Error(e);
                Task::none()
            }
            Message::ImageLoaded(index, result) => {
                if let AppState::DisplayingImages(cards) = &mut self.state {
                    if let Some(card) = cards.get_mut(index) {
                        match result {
                            Ok(bytes) => {
                                card.state = ImageState::Loaded(image::Handle::from_bytes(bytes));
                            }
                            Err(_) => {
                                card.state = ImageState::Error;
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::OpenModal(index) => {
                if let AppState::DisplayingImages(cards) = &self.state {
                    if let Some(card) = cards.get(index) {
                        let full_url = card.url.clone();
                        self.modal = Some(ModalState {
                            image_url: full_url.clone(),
                            thumb_url: card.thumb_url.clone(),
                            image_handle: None,
                            loading: true,
                        });
                        return Task::perform(fetch_image(full_url), Message::FullImageLoaded);
                    }
                }
                Task::none()
            }
            Message::CloseModal => {
                self.modal = None;
                Task::none()
            }
            Message::FullImageLoaded(result) => {
                if let Some(modal) = &mut self.modal {
                    modal.loading = false;
                    if let Ok(bytes) = result {
                        modal.image_handle = Some(image::Handle::from_bytes(bytes));
                    }
                }
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        // Header bar with input
        let header = container(
            row![
                text_input("Paste Fancaps URL...", &self.url_input)
                    .on_input(Message::UrlChanged)
                    .on_submit(Message::SubmitUrl)
                    .padding(12)
                    .width(Length::Fill)
                    .style(|_, _| text_input::Style {
                        background: Background::Color(BG_ELEVATED),
                        border: Border {
                            color: BORDER_COLOR,
                            width: 1.0,
                            radius: 4.0.into(),
                        },
                        icon: TEXT_MUTED,
                        placeholder: TEXT_MUTED,
                        value: TEXT_PRIMARY,
                        selection: ACCENT,
                    }),
                button(text("FETCH").size(14).color(BG_DARK))
                    .on_press(Message::SubmitUrl)
                    .padding([12, 24])
                    .style(|_, status| {
                        let base = button::Style {
                            background: Some(Background::Color(ACCENT)),
                            text_color: BG_DARK,
                            border: Border {
                                color: ACCENT,
                                width: 0.0,
                                radius: 4.0.into(),
                            },
                            ..Default::default()
                        };
                        match status {
                            button::Status::Hovered => button::Style {
                                background: Some(Background::Color(Color::from_rgb(
                                    0.0, 0.88, 0.92,
                                ))),
                                ..base
                            },
                            button::Status::Pressed => button::Style {
                                background: Some(Background::Color(Color::from_rgb(
                                    0.0, 0.68, 0.72,
                                ))),
                                ..base
                            },
                            _ => base,
                        }
                    }),
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center),
        )
        .padding(20)
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(BG_SURFACE)),
            border: Border {
                color: BORDER_COLOR,
                width: 0.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        });

        // Main content area
        let content: Element<'_, Message> = match &self.state {
            AppState::Idle => container(
                column![
                    text("FANCAP RIPPER").size(32).color(TEXT_PRIMARY),
                    text("Enter a Fancaps URL above to preview and download images")
                        .size(14)
                        .color(TEXT_MUTED),
                ]
                .spacing(8)
                .align_x(iced::Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into(),

            AppState::LoadingUrls => container(
                column![
                    text("FETCHING...").size(24).color(ACCENT),
                    text("Scanning page for images").size(14).color(TEXT_MUTED),
                ]
                .spacing(8)
                .align_x(iced::Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into(),

            AppState::Error(e) => container(
                column![
                    text("ERROR").size(24).color(Color::from_rgb(0.9, 0.3, 0.3)),
                    text(e).size(14).color(TEXT_MUTED),
                ]
                .spacing(8)
                .align_x(iced::Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into(),

            AppState::DisplayingImages(cards) => {
                // Create responsive grid - 4 columns
                let mut rows: Vec<Element<'_, Message>> = Vec::new();
                let mut current_row: Vec<Element<'_, Message>> = Vec::new();
                let columns_per_row = 4;

                for (i, card) in cards.iter().enumerate() {
                    let image_element: Element<'_, Message> = match &card.state {
                        ImageState::Loading => container(text("...").size(16).color(TEXT_MUTED))
                            .width(Length::Fill)
                            .height(Length::Fixed(180.0))
                            .center_x(Length::Fill)
                            .center_y(Length::Fill)
                            .style(|_| container::Style {
                                background: Some(Background::Color(BG_ELEVATED)),
                                ..Default::default()
                            })
                            .into(),

                        ImageState::Loaded(handle) => {
                            let idx = i;
                            mouse_area(
                                container(
                                    image(handle.clone())
                                        .width(Length::Fill)
                                        .content_fit(iced::ContentFit::Cover),
                                )
                                .width(Length::Fill)
                                .height(Length::Fixed(180.0))
                                .clip(true)
                                .style(|_| container::Style {
                                    background: Some(Background::Color(BG_ELEVATED)),
                                    ..Default::default()
                                }),
                            )
                            .on_press(Message::OpenModal(idx))
                            .into()
                        }

                        ImageState::Error => container(
                            text("FAILED")
                                .size(12)
                                .color(Color::from_rgb(0.9, 0.3, 0.3)),
                        )
                        .width(Length::Fill)
                        .height(Length::Fixed(180.0))
                        .center_x(Length::Fill)
                        .center_y(Length::Fill)
                        .style(|_| container::Style {
                            background: Some(Background::Color(BG_ELEVATED)),
                            ..Default::default()
                        })
                        .into(),
                    };

                    let card_element =
                        container(column![image_element]).width(Length::FillPortion(1));

                    current_row.push(card_element.into());

                    if current_row.len() >= columns_per_row || i == cards.len() - 1 {
                        while current_row.len() < columns_per_row {
                            current_row
                                .push(container(text("")).width(Length::FillPortion(1)).into());
                        }

                        let row_element = Row::from_vec(current_row).spacing(8).width(Length::Fill);
                        rows.push(row_element.into());
                        current_row = Vec::new();
                    }
                }

                let grid = Column::from_vec(rows).spacing(8).width(Length::Fill);

                // Status bar with pagination
                let loaded_count = cards
                    .iter()
                    .filter(|c| matches!(c.state, ImageState::Loaded(_)))
                    .count();
                let status_text = format!("{} / {} images loaded", loaded_count, cards.len());

                let prev_btn = if self.current_page > 1 {
                    button(text("<").size(14).color(TEXT_PRIMARY))
                        .on_press(Message::PrevPage)
                        .padding([8, 16])
                        .style(|_, status| nav_button_style(status))
                } else {
                    button(text("<").size(14).color(TEXT_MUTED))
                        .padding([8, 16])
                        .style(|_, _| button::Style {
                            background: Some(Background::Color(BG_ELEVATED)),
                            text_color: TEXT_MUTED,
                            border: Border::default(),
                            ..Default::default()
                        })
                };

                let next_btn = button(text(">").size(14).color(TEXT_PRIMARY))
                    .on_press(Message::NextPage)
                    .padding([8, 16])
                    .style(|_, status| nav_button_style(status));

                let status_bar = container(
                    row![
                        text(status_text).size(12).color(TEXT_MUTED),
                        row![
                            prev_btn,
                            text(format!("Page {}", self.current_page))
                                .size(12)
                                .color(TEXT_PRIMARY),
                            next_btn,
                        ]
                        .spacing(8)
                        .align_y(iced::Alignment::Center),
                    ]
                    .align_y(iced::Alignment::Center)
                    .spacing(20),
                )
                .padding([12, 20])
                .width(Length::Fill)
                .style(|_| container::Style {
                    background: Some(Background::Color(BG_SURFACE)),
                    border: Border {
                        color: BORDER_COLOR,
                        width: 0.0,
                        radius: 0.0.into(),
                    },
                    ..Default::default()
                });

                column![
                    status_bar,
                    scrollable(container(grid).padding(16).width(Length::Fill))
                        .width(Length::Fill)
                        .height(Length::Fill)
                ]
                .into()
            }
        };

        // Main layout with fixed header
        let main_content = column![header, content]
            .width(Length::Fill)
            .height(Length::Fill);

        // Wrap with modal if active
        let base: Element<'_, Message> = container(main_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(BG_DARK)),
                ..Default::default()
            })
            .into();

        if let Some(modal) = &self.modal {
            let modal_content: Element<'_, Message> = if modal.loading {
                container(text("Loading full image...").size(18).color(TEXT_PRIMARY))
                    .padding(40)
                    .style(|_| container::Style {
                        background: Some(Background::Color(BG_SURFACE)),
                        border: Border {
                            color: ACCENT,
                            width: 2.0,
                            radius: 8.0.into(),
                        },
                        ..Default::default()
                    })
                    .into()
            } else if let Some(handle) = &modal.image_handle {
                container(
                    column![
                        container(
                            image(handle.clone())
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .content_fit(iced::ContentFit::Contain)
                        )
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .clip(true),
                        container(
                            row![
                                text(&modal.image_url).size(10).color(TEXT_MUTED),
                                button(text("CLOSE").size(12).color(BG_DARK))
                                    .on_press(Message::CloseModal)
                                    .padding([8, 16])
                                    .style(|_, status| {
                                        let base = button::Style {
                                            background: Some(Background::Color(ACCENT)),
                                            text_color: BG_DARK,
                                            border: Border::default(),
                                            ..Default::default()
                                        };
                                        match status {
                                            button::Status::Hovered => button::Style {
                                                background: Some(Background::Color(
                                                    Color::from_rgb(0.0, 0.88, 0.92),
                                                )),
                                                ..base
                                            },
                                            _ => base,
                                        }
                                    }),
                            ]
                            .spacing(20)
                            .align_y(iced::Alignment::Center),
                        )
                        .padding(12)
                        .width(Length::Fill)
                        .style(|_| container::Style {
                            background: Some(Background::Color(BG_SURFACE)),
                            ..Default::default()
                        }),
                    ]
                    .width(Length::Fill)
                    .height(Length::Fill),
                )
                .width(Length::FillPortion(4))
                .height(Length::FillPortion(4))
                .style(|_| container::Style {
                    background: Some(Background::Color(BG_DARK)),
                    border: Border {
                        color: ACCENT,
                        width: 2.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                })
                .into()
            } else {
                container(text("Failed to load image").size(16).color(TEXT_MUTED))
                    .padding(40)
                    .style(|_| container::Style {
                        background: Some(Background::Color(BG_SURFACE)),
                        border: Border::default(),
                        ..Default::default()
                    })
                    .into()
            };

            let modal_overlay = mouse_area(
                container(opaque(modal_content))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .style(|_| container::Style {
                        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.85))),
                        ..Default::default()
                    }),
            )
            .on_press(Message::CloseModal);

            stack![base, modal_overlay].into()
        } else {
            base
        }
    }
}

fn nav_button_style(status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Background::Color(BG_ELEVATED)),
        text_color: TEXT_PRIMARY,
        border: Border {
            color: BORDER_COLOR,
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    };
    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(ACCENT)),
            text_color: BG_DARK,
            border: Border {
                color: ACCENT,
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        },
        _ => base,
    }
}

async fn fetch_urls(url: String) -> Result<Vec<FancapImage>, String> {
    let client = reqwest::Client::new();
    let user_agent = USER_AGENT;

    let res = client
        .get(&url)
        .header("User-Agent", user_agent)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let html = res.text().await.map_err(|e| e.to_string())?;
    let images = extract_image_urls(&html);

    if !images.is_empty() {
        return Ok(images);
    }

    Err("No images found on this page".to_string())
}

async fn fetch_image(url: String) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::new();
    let user_agent = USER_AGENT;
    let res = client
        .get(&url)
        .header("User-Agent", user_agent)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let bytes = res.bytes().await.map_err(|e| e.to_string())?;
    Ok(bytes.to_vec())
}
