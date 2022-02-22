pub trait Summary {
    fn summarize(&self) -> String {
        format!("(Read more from {})", self.summarize_author())
    }
    fn summarize_author(&self) -> String;
}

pub struct Tweet {
    pub username: String,
    pub content: String,
    pub reply: bool,
    pub retweet: bool
}

impl Summary for Tweet {
    fn summarize_author(&self) -> String {
        format!("@{}", self.username)
    }
}

pub struct NewsPaper {
    pub content: String
}

pub fn notify(item: &impl Summary) {
    println!("Breaking news! {}", item.summarize());
}