use aws_sdk_dynamodb::{
    model::{AttributeValue, DeleteRequest, WriteRequest},
    Client,
};
use chrono::{prelude::*, Duration};
use lambda_http::{http::StatusCode, Response};
use std::collections::HashMap;
use tracing::{info, instrument};
use uuid::Uuid;

use crate::{errors::Error, ext::AttributeValuesExt};

pub fn setup_tracing() {
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("failed to set tracing subscriber");
}

pub fn response(status_code: StatusCode, body: String) -> Response<String> {
    Response::builder()
        .status(status_code)
        .header("Content-Type", "application/json")
        .body(body)
        .unwrap()
}

pub struct SessionStore<'a> {
    table_name: String,
    expiration: i64,
    ddb: &'a Client,
}

impl SessionStore<'_> {
    pub fn new(ddb: &Client, table_name: String) -> SessionStore {
        SessionStore {
            table_name,
            expiration: 7 * 86400000,
            ddb,
        }
    }

    pub async fn get(&self, id: String) -> Result<Session, Error> {
        let res = self
            .ddb
            .get_item()
            .table_name(self.table_name.to_owned())
            .key(
                "PK",
                aws_sdk_dynamodb::model::AttributeValue::S(id.to_owned()),
            )
            .send()
            .await?;

        match res.item {
            Some(item) => Ok(item.try_into()?),
            None => Err(Error::new("Session does not exist.")),
        }
    }

    pub async fn create(&self, username: String) -> Result<String, Error> {
        self.create_at(username, Utc::now()).await
    }

    pub async fn create_at(
        &self,
        username: String,
        created_at: DateTime<Utc>,
    ) -> Result<String, Error> {
        let id = Uuid::new_v4();
        let session = &Session {
            id: id.to_string(),
            username: username.clone(),
            created_at,
            expires_at: created_at + Duration::seconds(self.expiration),
        };

        self.ddb
            .put_item()
            .table_name(self.table_name.to_owned())
            .set_item(Some(session.into()))
            .send()
            .await?;

        Ok(id.to_string())
    }

    #[instrument(skip(self))]
    pub async fn delete_user_sessions(&self, username: String) -> Result<(), Error> {
        let res = self
            .ddb
            .query()
            .table_name(self.table_name.clone())
            .index_name("GSI1")
            .key_condition_expression("#username = :username".to_owned())
            .expression_attribute_names("#username".to_owned(), "GSI1PK".to_owned())
            .expression_attribute_values(
                ":username".to_owned(),
                AttributeValue::S(username.to_owned()),
            )
            .send()
            .await?;

        info!("{} sessions found for {}", res.count(), username);

        let deletes: Vec<WriteRequest> = res
            .items
            .unwrap_or_default()
            .into_iter()
            .map(|item| {
                let hk = item.get("PK").unwrap().to_owned();
                WriteRequest::builder()
                    .delete_request(DeleteRequest::builder().key("PK".to_owned(), hk).build())
                    .build()
            })
            .collect();

        let batch_request = self.ddb.batch_write_item();
        batch_request
            .request_items(self.table_name.clone(), deletes)
            .send()
            .await?;

        Ok(())
    }
}

pub struct Session {
    pub id: String,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    pub username: String,
}

impl Session {
    pub fn is_expired(self) -> bool {
        self.expires_at <= Utc::now()
    }
}

impl From<&Session> for HashMap<String, AttributeValue> {
    fn from(value: &Session) -> Self {
        let mut retval = HashMap::new();
        // Indexing attributes
        retval.insert("PK".to_owned(), AttributeValue::S(value.id.to_owned()));
        retval.insert(
            "GSI1PK".to_owned(),
            AttributeValue::S(value.username.to_owned()),
        );
        retval.insert(
            "TTL".to_owned(),
            AttributeValue::N(value.expires_at.timestamp().to_string()),
        );
        // Item attributes
        retval.insert("id".to_owned(), AttributeValue::S(value.id.to_owned()));
        retval.insert(
            "created_at".to_owned(),
            AttributeValue::S(value.created_at.to_rfc3339()),
        );
        retval.insert(
            "expires_at".to_owned(),
            AttributeValue::S(value.expires_at.to_rfc3339()),
        );
        retval.insert(
            "username".to_owned(),
            AttributeValue::S(value.username.to_owned()),
        );

        retval
    }
}

impl TryFrom<HashMap<String, AttributeValue>> for Session {
    type Error = Error;
    fn try_from(value: HashMap<String, AttributeValue>) -> Result<Self, Self::Error> {
        Ok(Session {
            id: value.get_s("id").ok_or(Error::new("missing id"))?,
            created_at: value
                .get_dt("created_at")
                .ok_or(Error::new("missing created_at date"))?,
            expires_at: value
                .get_dt("expires_at")
                .ok_or(Error::new("missing expires_at date"))?,
            username: value
                .get_s("username")
                .ok_or(Error::new("missing username"))?,
        })
    }
}
