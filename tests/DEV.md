Can create containers for the test scenarios like so:

```rust
mod common;
use common::*;
use crate::common::models::URLShortenerWorld;

let mut world = URLShortenerWorld::default();

let env: Option<Vec<String>> = Some(vec![
    "POSTGRES_PASSWORD=mysecretpassword".to_string(),
]);

let (container_name, container_port) = utility::create_and_start_docker_container(
    "postgres", // image name
    "5432", // internal container port
    // expected message to see when the container is running and ready
    "database system is ready to accept connections",
    env // optional parameter
)
    .await;

utility::stop_docker_container(&*world.container_name).await;
```

To test locally:
```rust
.before(|_feature, _rule, _scenario, _world| {
    Box::pin(async move {
        // converts async block of code into a future

        // spins up a postgres docker container and a container for the application/microservice that is under test

        let db_user = "postgres";
        let db_pw = "postgres";
        let db_name = "url-shortener-db";

        let env: Option<Vec<String>> = Some(vec![
            format!("POSTGRES_PASSWORD={}", db_pw),
            format!("POSTGRES_USER={}", db_user),
            format!("POSTGRES_DB={}", db_name),
        ]);

        let (postgres_container_name, postgres_container_port) =
            utility::create_and_start_docker_container(
                "postgres",
                "5432",
                "database system is ready to accept connections",
                env,
            )
            .await;

        _world.db_port = postgres_container_port;
        _world.db_container_name = postgres_container_name;

        let env: Option<Vec<String>> = Some(vec![
            "DB_HOST=host.docker.internal".to_string(),
            format!("DB_USER={}", db_user).to_string(),
            format!("DB_PASSWORD={}", db_pw).to_string(),
            format!("DB_NAME={}", db_name).to_string(),
            format!("DB_PORT={}", _world.db_port).to_string(),
        ]);

        // let (container_name, container_port) = utility::create_and_start_docker_container(
        //     "url_shortener_rust",
        //     "8080",
        //     "The server has been started",
        //     env,
        // )
        // .await;

        // can change the container name to localhost and port to 8080 to test without containers
        _world.url_shortener_container_name = "localhost".to_string();
        _world.url_shortener_container_host_port = 8080;
        // _world.url_shortener_container_name = container_name;
        // _world.url_shortener_container_host_port = container_port;
    })
})
.after(|_feature, _rule, _scenario, _ev, _world| {
    Box::pin(async move {
        if let Some(world) = _world {
            // utility::stop_docker_container(&world.url_shortener_container_name).await;
            utility::stop_docker_container(&world.db_container_name).await;
        }
    })
})
```