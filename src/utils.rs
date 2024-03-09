use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

pub fn gen_random_string() -> String {
    let rand_string: Vec<u8> = thread_rng().sample_iter(&Alphanumeric).take(30).collect();

    String::from_utf8(rand_string).unwrap()
}
