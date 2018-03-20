#![feature(plugin)]
#![feature(custom_derive)]
#![plugin(rocket_codegen)]
extern crate rocket;
// #[macro_use]
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
// use std::fs::File;
use std::sync::Mutex;
use rocket::response::NamedFile;
// use std::io::prelude::*;
// use rocket::http::RawStr;
use rocket::State;
use rocket_contrib::Json;
// use rocket::response::Redirect;
// use rocket::request::Form;

const LOWER_ALPHA_SPACE_CHARS: &str = "abcdefghijklmnopqrstuvwxyz ";
const LOWER_ALPHANUMERIC_CHARS: &str = "abcdefghijklmnopqrstuvwxyz0123456789";
const LOWER_ALPHANUMERIC_SYMBOLS_CHARS: &str = "abcdefghijklmnopqrstuvwxyz0123456789!? ,;.:-_()[]{}&%$";

fn contains_only(s: &str, chars: &str) -> bool{
    s.chars().all(|c| chars.chars().any(|ac| ac==c))
}

struct Poll {
    number: isize,
    title: String,
    questions: Vec<String>,
    answers: Vec<(String, Vec<String>)>,
}

#[get("/")]
fn index() -> io::Result<NamedFile> {
    NamedFile::open("frontend/index.html")
}

#[get("/<file..>")]
fn files(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("resources/").join(file)).ok()
}

#[post("/poll_name_exists", format = "application/json", data = "<name>")]
fn poll_name_exists(polls: State<Mutex<HashMap<String, Poll>>>, name: Json<String>) -> Json<bool>{
    let mut name=name.into_inner();
    name.make_ascii_lowercase();
    match polls.lock() {
        Err(_) => Json(true),
        Ok(polls) => Json(polls.contains_key(&name)),
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ReceivedPoll {
    name: String,
    number: isize,
    title: String,
    questions: String,
}

#[post("/start_poll", format = "application/json", data = "<received_poll>")]
fn start_poll(polls: State<Mutex<HashMap<String, Poll>>>, received_poll: Json<ReceivedPoll>) -> Json<&str>{
    let mut received_poll=received_poll.into_inner();
    received_poll.name.make_ascii_lowercase();
    received_poll.title.make_ascii_lowercase();
    let ReceivedPoll{name, number, title, questions}=received_poll;
    let questions=questions
    .lines()
    .map(|l| {
        let mut l=l.to_string();
        l.make_ascii_lowercase();
        l
    })
    .collect::<Vec<String>>();


    if name.len() < 3 || name.len() > 32 || !contains_only(name.as_str(), LOWER_ALPHANUMERIC_CHARS){
        return Json("Name length must be between 3 and 32 characters long and only contain alphanumeric characters.")
    }

    if number < 1 || number > 1000{
        return Json("Number must be between 1 and 1000.")
    }

    if title.len() < 3 || title.len() > 32 || !contains_only(title.as_str(), LOWER_ALPHANUMERIC_SYMBOLS_CHARS){
        return Json("Title length must be between 3 and 32 characters long and only contain alphanumeric or these `!? ,;.:-_()[]{}&%$` characters.")
    }

    if questions.len()!=0 && (questions.len() < 3 || questions.len() > 1000){
        return Json("There must be between 3 and 1000 questions.")
    }

    if questions.iter().any(|q| (*q).len() < 3 || (*q).len() > 32 || !contains_only((*q).as_str(), LOWER_ALPHA_SPACE_CHARS)){
        return Json("The length of each question must be between 3 and 32, and only contain letters and spaces.")
    }

    match polls.lock() {
        Err(_) => Json("Server error."),
        Ok(mut polls) => {
            if polls.contains_key(&name){
                Json("A poll with that name already exists.")
            } else {
                let poll=Poll{number: number, title: title, questions: questions, answers: vec![]};
                polls.insert(name, poll);
                Json("success")
            }
        },
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct UserAndPollName {
    user_name: String,
    poll_name: String,
}

#[post("/has_user_answered_poll", format = "application/json", data = "<user_and_poll_name>")]
fn has_user_answered_poll(polls: State<Mutex<HashMap<String, Poll>>>, user_and_poll_name: Json<UserAndPollName>) -> Json<bool>{
    let mut user_and_poll_name=user_and_poll_name.into_inner();
    user_and_poll_name.user_name.make_ascii_lowercase();
    user_and_poll_name.poll_name.make_ascii_lowercase();

    match polls.lock() {
        Err(_) => Json(true),
        Ok(polls) => {
            match polls.get(&user_and_poll_name.poll_name) {
                None => Json(true),
                Some(poll) => Json(poll.answers.iter().any(|a| (*a).0==user_and_poll_name.user_name)),
            }
        },
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct PollTitleAndQuestions {
    title: String,
    questions: Vec<String>,
}

#[post("/get_poll", format = "application/json", data = "<name>")]
fn get_poll(polls: State<Mutex<HashMap<String, Poll>>>, name: Json<String>) -> Json<Option<PollTitleAndQuestions>>{
    let mut name=name.into_inner();
    name.make_ascii_lowercase();

    match polls.lock() {
        Err(_) => Json(None),
        Ok(polls) => match polls.get(&name) {
            None => Json(None),
            Some(poll) => Json(Some(PollTitleAndQuestions{title: poll.title.clone(), questions: poll.questions.clone()})),
        },
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct PollResponse {
    user_name: String,
    poll_name: String,
    answers: Vec<isize>,
}

#[post("/fill_poll", format = "application/json", data = "<poll_response>")]
fn fill_poll(polls: State<Mutex<HashMap<String, Poll>>>, poll_response: Json<PollResponse>) -> Json<&str>{
    let mut poll_response=poll_response.into_inner();
    poll_response.user_name.make_ascii_lowercase();
    poll_response.poll_name.make_ascii_lowercase();
    let PollResponse{user_name, poll_name, answers}=poll_response;

    if user_name.len() < 3 || user_name.len() > 32 || !contains_only(user_name.as_str(), LOWER_ALPHANUMERIC_CHARS){
        return Json("User name length must be between 3 and 32 characters long and only contain alphanumeric characters.")
    }

    if poll_name.len() < 3 || poll_name.len() > 32 || !contains_only(poll_name.as_str(), LOWER_ALPHANUMERIC_CHARS){
        return Json("Poll name length must be between 3 and 32 characters long and only contain alphanumeric characters.")
    }

    match polls.lock() {
        Err(_) => Json("Server error."),
        Ok(mut polls) => match polls.get_mut(&poll_name) {
            None => Json("A poll with that name does not exists."),
            Some(mut poll) => {
                if poll.answers.iter().any(|a| (*a).0==user_name){
                    Json("A user with that name has already answered the poll.")
                } else if poll.questions.len()!=answers.len(){
                    Json("The number of poll questions and answers is different.")
                } else {
                    let answers=answers.into_iter().map(|a| if a==2 {"y".to_string()} else if a==1 {"o".to_string()} else {"n".to_string()}).collect::<Vec<String>>();
                    poll.answers.push((user_name, answers));
                    Json("success")
                }
            },
        },
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct FreeEntryPollResponse {
    user_name: String,
    poll_name: String,
    answers: String,
}

#[post("/fill_free_entry_poll", format = "application/json", data = "<poll_response>")]
fn fill_free_entry_poll(polls: State<Mutex<HashMap<String, Poll>>>, poll_response: Json<FreeEntryPollResponse>) -> Json<&str>{
    let mut poll_response=poll_response.into_inner();
    poll_response.user_name.make_ascii_lowercase();
    poll_response.poll_name.make_ascii_lowercase();
    let FreeEntryPollResponse{user_name, poll_name, answers}=poll_response;
    let answers=answers
    .lines()
    .map(|l| {
        let mut l=l.to_string();
        l.make_ascii_lowercase();
        l
    })
    .collect::<Vec<String>>();

    if user_name.len() < 3 || user_name.len() > 32 || !contains_only(user_name.as_str(), LOWER_ALPHANUMERIC_CHARS){
        return Json("User name length must be between 3 and 32 characters long and only contain alphanumeric characters.")
    }

    if poll_name.len() < 3 || poll_name.len() > 32 || !contains_only(poll_name.as_str(), LOWER_ALPHANUMERIC_CHARS){
        return Json("Poll name length must be between 3 and 32 characters long and only contain alphanumeric characters.")
    }

    if answers.len() < 2 || answers.len() > 1000{
        return Json("There must be between 2 and 1000 answers.")
    }

    if answers.iter().any(|a| (*a).len() < 3 || (*a).len() > 32 || !contains_only((*a).as_str(), LOWER_ALPHA_SPACE_CHARS)){
        return Json("The length of each answer must be between 3 and 32, and only contain letters and spaces.")
    }

    match polls.lock() {
        Err(_) => Json("Server error."),
        Ok(mut polls) => match polls.get_mut(&poll_name) {
            None => Json("A poll with that name does not exists."),
            Some(mut poll) => {
                if poll.answers.iter().any(|a| (*a).0==user_name){
                    Json("A user with that name has already answered the poll.")
                } else if poll.questions.len()!=answers.len(){
                    Json("The number of poll questions and answers is different.")
                } else {
                    poll.answers.push((user_name, answers));
                    Json("success")
                }
            },
        },
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct PollResult {
    poll_name: String,
    user_names: Vec<String>,
    all_yay: Vec<String>,
    some_yay_others_open: Vec<String>,
    error_string: String,
}

#[post("/get_poll_results", format = "application/json", data = "<name>")]
fn get_poll_results(polls: State<Mutex<HashMap<String, Poll>>>, name: Json<String>) -> Json<PollResult>{
    // let mut name=name.into_inner();
    // name.make_ascii_lowercase();
    // match polls.lock() {
    //     Err(_) => Json(true),
    //     Ok(polls) => Json(polls.contains_key(&name)),
    // }
    Json(PollResult{poll_name: "".into(), user_names: vec![], all_yay: vec![], some_yay_others_open: vec![], error_string: "".into()})
}

fn main() {
    let polls:HashMap<String, Poll>=HashMap::new();
    rocket::ignite()
    .manage(Mutex::new(polls))
    .mount("/", routes![index, files])
    .launch();
}
