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