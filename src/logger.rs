use tracing::{level_filters::LevelFilter, Level};
use tracing_appender::rolling::RollingFileAppender;
use tracing_subscriber::fmt::{SubscriberBuilder, format::{DefaultFields, Format}};

pub fn file_logger(log_name: &str) -> SubscriberBuilder<DefaultFields, Format, LevelFilter, RollingFileAppender> {
    let file_appender = tracing_appender::rolling::never("log", log_name);
    let file_logger = tracing_subscriber::fmt()
        .with_writer(file_appender).with_max_level(Level::DEBUG);
    file_logger
}