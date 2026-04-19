pub struct Article {
    pub id: u32,
    pub url: String,
    pub title: String,
    pub section: Option<String>,
    pub author: Option<String>,
    pub published_at: String,
    pub snippet: Option<String>,
}

#[expect(dead_code)]
pub struct FullArticle {
    pub id: u32,
    pub url: String,
    pub title: String,
    pub section: Option<String>,
    pub author: Option<String>,
    pub published_at: String,
    pub content_text: Option<String>,
    pub images: Vec<ArticleImage>,
}

#[expect(dead_code)]
pub struct ArticleImage {
    pub id: u32,
    pub url: String,
    pub alt_text: Option<String>,
    pub data: Option<Vec<u8>>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

pub struct NewArticle<'a> {
    pub url: &'a str,
    pub title: &'a str,
    pub section: Option<&'a str>,
    pub author: Option<&'a str>,
    pub published_at: &'a str,
    pub content_text: Option<&'a str>,
    pub content_html: Option<&'a str>,
}

pub struct NewImage<'a> {
    pub article_id: u32,
    pub url: &'a str,
    pub alt_text: Option<&'a str>,
    pub data: Option<&'a [u8]>,
    pub format: Option<&'a str>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}
