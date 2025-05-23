[Cucumber doc](https://cucumber-rs.github.io/cucumber/current/quickstart.html)

https://docs.rs/crate/reqwest/latest

[Reqwest](https://docs.rs/reqwest/latest/reqwest/)

[Log crate](https://docs.rs/log/latest/log/index.html)
- https://github.com/borntyping/rust-simple_logger

```bash
cargo test --test url_shortener_steps
```
----

spin up a docker image and tear it down before each scenario

--- 
## Requirements:

need to have a docker image for the url_shortener:
```bash
docker build -t url_shortener_rust .
```


clean containers
```bash
docker stop $(docker ps -a -q) && docker container prune
```

---

The specs now create a new docker container for each scenario and stops the container after each scenario


```text
[[bin]]
name = "url-shortener-cucumber"
path = "src/main.rs"
test = false  # This prevents testing main.rs
```


```bash
cargo run --bin url-shortener-cucumber
cargo test
```