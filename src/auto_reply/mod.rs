pub mod commands;
pub mod directives;
pub mod dispatcher;
pub mod reply;
pub mod traits;

pub use commands::HelpCommand;
pub use directives::DefaultDirectiveParser;
pub use dispatcher::DefaultDispatcher;
pub use reply::DefaultReplyDispatcher;
pub use traits::{
    CommandContext, CommandResult, DirectiveParser, DispatchContext, DispatchResult, Dispatcher,
    InboundMessage, ParsedDirective, ParsedMessage, ReplyContext, ReplyDispatcher, ReplyMessage,
    SlashCommandHandler,
};

pub fn create_directive_parser() -> Box<dyn DirectiveParser> {
    Box::new(DefaultDirectiveParser)
}

pub fn create_dispatcher(
    parser: Box<dyn DirectiveParser>,
    commands: Vec<Box<dyn SlashCommandHandler>>,
) -> Box<dyn Dispatcher> {
    Box::new(DefaultDispatcher::new(parser, commands))
}

pub fn create_reply_dispatcher() -> Box<dyn ReplyDispatcher> {
    Box::new(DefaultReplyDispatcher)
}
