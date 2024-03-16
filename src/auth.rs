use std::cell::RefCell;
use std::rc::Rc;

use regex::Regex;

use crate::auth::AuthResult::{NotAuthorized, PasswordValidated, SourceValidated};
use crate::config::config::{Config, SecurityMode};

pub struct Authorization {
    config: Rc<RefCell<Config>>,
    pwd_regex: Regex,
}

pub enum SourceValidityResult {
    OwnerPrivateMessage,
    OwnerAnnounceChannel,
    OwnerPublicChannel,
    AnnounceChannel,
    InvalidSource,
}

pub enum AuthResult {
    PasswordValidated,
    SourceValidated,
    NotAuthorized,
}

pub enum MessageTypes {
    Command,
    Announcement,
    Other,
}

impl Authorization {
    pub fn new(config: Rc<RefCell<Config>>) -> Self {
        Self { config, pwd_regex: Regex::new(r"auth:\[(?P<password>.+)]").unwrap() }
    }

    pub fn authenticate(&self, nick: &str, channel: &str, message: &str, message_type: MessageTypes) -> AuthResult {
        match message_type {
            MessageTypes::Command => {
                if self.config.borrow().is_commands_enabled() {
                    return self.check_security_mode(nick, channel, message);
                }
            }
            MessageTypes::Announcement => {
                if let SourceValidityResult::AnnounceChannel = self.validate_source(nick, channel) {
                    return SourceValidated;
                }
            }
            MessageTypes::Other => {
                return NotAuthorized;
            }
        }
        NotAuthorized
    }

    fn check_security_mode(&self, nick: &str, channel: &str, message: &str) -> AuthResult {
        match self.config.borrow().get_security_mode() {
            SecurityMode::IrcUserName(ref u) => {
                if let SourceValidityResult::OwnerPrivateMessage = self.validate_source(nick, channel) {
                    return SourceValidated;
                }
            }
            SecurityMode::Password(ref p) => {
                if let Some(caps) = self.pwd_regex.captures(message) {
                    let password = &caps["password"];
                    if password == p {
                        return PasswordValidated;
                    }
                }
            }
        }
        NotAuthorized
    }

    pub fn validate_source(&self, nick: &str, channel: &str) -> SourceValidityResult {
        let is_owner = self.is_owner(nick);
        let is_valid_channel = self.is_valid_channel(channel);
        if is_owner && is_valid_channel {
            return SourceValidityResult::OwnerAnnounceChannel;
        } else if is_owner && nick.eq(channel) {
            return SourceValidityResult::OwnerPrivateMessage;
        } else if is_owner && !is_valid_channel {
            return SourceValidityResult::OwnerPublicChannel;
        } else if !is_owner && is_valid_channel {
            return SourceValidityResult::AnnounceChannel;
        }
        SourceValidityResult::InvalidSource
    }

    fn is_valid_channel(&self, channel: &str) -> bool {
        let channels = self.config.borrow().get_irc_config().channels.clone();
        channels.contains(&channel.to_string())
    }

    fn is_owner(&self, nick: &str) -> bool {
        if let SecurityMode::IrcUserName(valid_user) = self.config.borrow().get_security_mode() {
            if nick.eq(&valid_user) {
                return true;
            }
        }
        false
    }

}