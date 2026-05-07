use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)] // this statements help to automatically creates necessary function needed to safely transfer owenrhip of the object of this struct
pub struct EnrollmentRequest {
    pub agent_id: String,
    pub public_key: String,
}// when agent erolls with server. it sends afent_id, public_key as payload

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct EnrollmentResponse {
    pub status: String,
    pub certificate: Option<String>, // std::optional<std::string>
}

#[cfg(test)]
use mockall::automock; // only import this during testing.

#[cfg_attr(test, automock)]
pub trait Discovery {
    fn discover_server(&self) -> Result<String, String>;
}
/*class Discovery {
public:
    virtual Result discoverServer() = 0;
};
fn discover_server(&self) -> discoverServer() const
&self: borrow object, don't take ownership, read-only access
Result<String, String>; -> std::expected<std::string, std::string>
*/

#[cfg_attr(test, automock)] // "When compiling tests, auto-generate a mock version." (Rust feature)
pub trait EnrollmentClient { // "Something capable of sending enrollment request."
    fn enroll(&self, request: EnrollmentRequest) -> Result<EnrollmentResponse, String>;
}
/* notice how EnrollmentRequest is used intead of & EnrollmentRequest meaning the ownership 
moves into function.
WHY Move Ownership?

Usually because:

request is consumed
no need to retain caller ownership
avoids unnecessary copies

Very common Rust optimization pattern.

*/
#[cfg(test)]
mod tests { // "Can agent connect to enrollment server on port 8443?"
    use super::*; // using namespace parent_module;
    use reqwest;

    #[tokio::test]
    async fn test_agent_connection_to_server_port_8443() {
        let client = reqwest::Client::new();
        let res = client.get("http://localhost:8443/enroll").send().await;
        assert!(res.is_ok(), "Agent failed to send request to the server");
        let res = res.unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK, "Server did not return 200 OK");
        let body = res.text().await;
        assert!(body.is_ok(), "Failed to get response body");
        assert_eq!(body.unwrap(), "Enrollment successful", "Server response did not match expected message");
    }

}
