/*struct Subscribers {
    subscribers: Mutex<HashSet<Arc<dyn Subscriber>>>,
}*/

use std::rc::Rc;

pub trait Publisher {
    fn add_subscriber(&self, subscriber: Rc<dyn Subscriber>);
    fn notify_subscribers(&self, message: &str);
}

pub trait Subscriber {
    fn handle_notification(&mut self, message: &str);
}

