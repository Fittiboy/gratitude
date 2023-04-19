use crate::message::registered_users;
use async_std::task::block_on;

use crate::message::User;
use worker::*;

#[durable_object]
pub struct Userlist {
    env: Env,
}

#[durable_object]
impl DurableObject for Userlist {
    fn new(state: State, env: Env) -> Self {
        Self { env }
    }

    async fn fetch(&mut self, mut req: Request) -> Result<Response> {
        let kv = self
            .env
            .kv("grateful_users")
            .expect("Worker should have access to grateful_users binding");
        let mut users = block_on(registered_users(&kv));
        match req.method() {
            Method::Get => Response::ok(&format!("{}", serde_json::to_string(&users).unwrap())),
            Method::Post => {
                let user = block_on(req.json::<User>())
                    .expect("Should always be passed a User from the worker");
                if users
                    .iter()
                    .find(|local_user| user.uid == local_user.uid)
                    .is_some()
                {
                    return Response::error("User already registered", 409);
                } else {
                    users.push(user);
                    if let Err(err) = block_on(kv.put("users", users).unwrap().execute()) {
                        console_error!("Couldn't add user to list: {}", err);
                        return Response::error("Something went wrong", 500);
                    }
                }
                Response::ok("".to_string())
            }
            Method::Delete => {
                let user = block_on(req.json::<User>())
                    .expect("Should always be passed a User from the worker");
                if users
                    .iter()
                    .find(|local_user| user.uid == local_user.uid)
                    .is_none()
                {
                    return Response::error("User not registered", 409);
                } else {
                    let original_length = users.len();
                    users.retain(|local_user| user.uid != local_user.uid);
                    let length_after = users.len();
                    if !(original_length - 1 == length_after) {
                        return Response::error(
                            format!(
                                "Length after removing not one less. Old: {}, New: {}",
                                original_length, length_after
                            ),
                            500,
                        );
                    } else {
                        if let Err(err) = block_on(kv.put("users", users).unwrap().execute()) {
                            console_error!("Couldn't remove user from list: {}", err);
                            return Response::error("Something went wrong", 500);
                        }
                    }
                }
                Response::ok("".to_string())
            }
            _ => unimplemented!("Other request methods are not needed"),
        }
    }
}
