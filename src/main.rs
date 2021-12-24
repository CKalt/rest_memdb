mod db_access;

use actix_web::{web, web::Path, App, HttpRequest, HttpResponse, 
        HttpServer, Responder};
use serde_derive::Deserialize;
use std::sync::Mutex;

use std::fs::File;
use daemonize::Daemonize;

struct AppState {
    db: db_access::DbConnection,
}

fn get_all_persons_ids(state: web::Data<Mutex<AppState>>) -> impl Responder {
    println!("In get_all_persons_ids");
    let db_conn = &state.lock().unwrap().db;
    db_conn
        .get_all_persons_ids()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn get_person_name_by_id(
    state: web::Data<Mutex<AppState>>,
    info: Path<(String,)>,
) -> impl Responder {
    println!("In get_person_name_by_id");
    let id = &info.0;
    let id = id.parse::<u32>();
    if id.is_err() {
        return HttpResponse::NotFound().finish();
    }
    let id = id.unwrap();
    let db_conn = &state.lock().unwrap().db;
    if let Some(name) = db_conn.get_person_name_by_id(id) {
        HttpResponse::Ok().content_type("text/plain").body(name)
    } else {
        HttpResponse::NotFound().finish()
    }
}

#[derive(Deserialize)]
pub struct Filter {
    partial_name: Option<String>,
}

fn get_persons(state: web::Data<Mutex<AppState>>, query: web::Query<Filter>) -> impl Responder {
    println!("In get_persons");
    let db_conn = &state.lock().unwrap().db;
    db_conn
        .get_persons_id_and_name_by_partial_name(
            &query.partial_name.clone().unwrap_or_else(|| "".to_string()),
        )
        .map(|p| p.0.to_string() + ": " + &p.1)
        .collect::<Vec<_>>()
        .join("; ")
}

fn insert_person(state: web::Data<Mutex<AppState>>, info: Path<(String,)>) -> impl Responder {
    println!("In insert_person");
    let name = &info.0;
    let db_conn = &mut state.lock().unwrap().db;
    format!("{}", db_conn.insert_person(name))
}

fn invalid_resource(req: HttpRequest) -> impl Responder {
    println!("Invalid URI: \"{}\"", req.uri());
    HttpResponse::NotFound()
}

fn web_server() -> std::io::Result<()> {
    let server_address = "127.0.0.1:8080";
    println!("Listening at address {}

To Test:
curl -X POST http://localhost:8080/person/George
curl -X GET http://localhost:8080/persons
curl -X GET http://localhost:8080/person/name_by_id/1", server_address);
    let db_conn = web::Data::new(Mutex::new(AppState {
        db: db_access::DbConnection::new(),
    }));

    HttpServer::new(move || {
        App::new()
            .register_data(db_conn.clone())
            .service(web::resource("/persons/ids").route(web::get().to(get_all_persons_ids)))
            .service(
                web::resource("/person/name_by_id/{id}")
                    .route(web::get().to(get_person_name_by_id)),
            )
            .service(web::resource("/persons").route(web::get().to(get_persons)))
            .service(web::resource("/person/{name}").route(web::post().to(insert_person)))
            .default_service(web::route().to(invalid_resource))
    })
    .bind(server_address)?
    .run()
}

fn main() -> std::io::Result<()> {
    let stdout = File::create("daemon.out").unwrap();
    let stderr = File::create("daemon.err").unwrap();

    let daemonize = Daemonize::new()
        .pid_file("test.pid")  // Every method except `new` and             `start`
        .chown_pid_file(false) // is optional, see `Daemonize` documentation
        .working_directory("./") // for default behaviour.
        .stdout(stdout)  // Redirect stdout to `/home/chris/projects/rust/daemon/daemon.out`.
        .stderr(stderr)  // Redirect stderr to `/home/chris/projects/rust/daemon/daemon.err`.
        .privileged_action(|| "Executed before drop privileges");

    match daemonize.start() {
        Ok(_) => {
            println!("Success, daemonized");
            web_server()?
        },
        Err(e) => eprintln!("Error, {}", e),
    }
    Ok(())
}
