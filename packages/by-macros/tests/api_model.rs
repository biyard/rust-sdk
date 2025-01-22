#[cfg(feature = "server")]
use by_axum::aide;

use by_macros::api_model;

type Result<T> = std::result::Result<T, by_types::ApiError<String>>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
#[cfg_attr(feature = "server", derive(schemars::JsonSchema, aide::OperationIo))]
#[api_model(base = "/topics/v1", iter_type=Vec, table = topics)] // rename is supported but usually use default(snake_case)
pub struct Topic {
    #[api_model(summary, primary_key)]
    pub id: String,
    #[api_model(read_action = user_info)]
    pub wallet_address: String,
    #[api_model(read_action = [check_email,user_info])]
    pub email: String,
    #[api_model(summary, action = create)]
    pub title: String,
    #[api_model(summary, queryable, query_action = search_by, action = create, action_by_id = update)]
    pub description: String,
    #[api_model(summary, queryable, action_by_id = update, read_action = user_info)]
    pub status: i32,
    #[api_model(summary, query_action = [search_by, date_from], type = TIMESTAMP)]
    pub created_at: i64,
    #[api_model(many_to_many = topic_user_likes, foreign_table_name = users, foreign_key = id, foreign_key_type = TEXT)]
    pub is_liked: bool,

    #[api_model(type = TIMESTAMP)]
    pub updated_at: i64,
    #[api_model(action_by_id = like, related = CommentRequest)]
    #[api_model(action = comment, related = Comment)]
    #[api_model(one_to_many = comments, foreign_key = topic_id)]
    pub comments: Vec<Comment>,

    #[api_model(action_by_id = update)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "server", derive(schemars::JsonSchema, aide::OperationIo))]
#[api_model(base = "/topics/v1/:topic-id/comments", iter_type=Vec)]
pub struct Comment {
    pub id: String,
    #[api_model(action = comment, related = String, read_action = search_by)]
    pub content: String,
    #[api_model(action_by_id = update, related = i64)]
    pub updated_at: i64,

    #[api_model(many_to_one = topics, foreign_key = id, foreign_key_type = TEXT)]
    pub topic_id: String,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "server", derive(schemars::JsonSchema, aide::OperationIo))]
#[api_model(base = "/topics/v1/categories", iter_type=Vec, table = categories)]
pub struct Category {
    pub id: String,
    pub topic_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "server", derive(schemars::JsonSchema, aide::OperationIo))]
pub struct CommentRequest {
    pub comment_id: String,
    pub is_liked: bool,
}

#[test]
fn test_macro_expansion_topic() {
    let _read_action = TopicReadAction {
        action: Some(TopicReadActionType::CheckEmail),
        wallet_address: None,
        email: Some("test".to_string()),
        status: Some(1),
    };

    let _ = TopicReadAction::new().user_info("wallet".to_string(), "email".to_string(), 1);
    let _ = TopicReadAction::new().check_email("email".to_string());

    let q = TopicQuery {
        action: Some(TopicQueryActionType::DateFrom),
        size: 10,
        bookmark: None,
        description: None,
        status: Some(1),
        created_at: None,
    };

    assert_eq!(q.status, Some(1));
    assert_eq!(q.size, 10);
    assert_eq!(q.bookmark, None);
    assert_eq!(q.description, None);

    let _q = TopicQuery::new(10)
        .with_bookmark("test".to_string())
        .search_by("test".to_string(), 0);

    let q = TopicQuery::new(10)
        .with_bookmark("test".to_string())
        .date_from(1);

    let summary = TopicSummary::default();
    assert_eq!(summary.id, "".to_string());
    assert_eq!(summary.title, "".to_string());
    assert_eq!(summary.description, "".to_string());
    assert_eq!(summary.status, 0);
    assert_eq!(summary.created_at, 0);

    let create_request = TopicCreateRequest {
        title: "title".to_string(),
        description: "description".to_string(),
    };
    assert_eq!(create_request.title, "title".to_string());
    assert_eq!(create_request.description, "description".to_string());

    let update_request = TopicUpdateRequest {
        description: "description".to_string(),
        status: 1,
        tags: vec!["tag".to_string()],
    };
    assert_eq!(update_request.description, "description".to_string());
    assert_eq!(update_request.status, 1);
    assert_eq!(update_request.tags, vec!["tag".to_string()]);

    let create_request = TopicAction::Create(create_request);
    let update_request = TopicByIdAction::Update(update_request);
    let like_request = TopicByIdAction::Like(CommentRequest {
        comment_id: "1".to_string(),
        is_liked: true,
    });
    let comment_request = TopicAction::Comment(Comment {
        id: "1".to_string(),
        content: "content".to_string(),
        updated_at: 0,
        topic_id: "1".to_string(),
    });

    let cli = Topic::get_client("");
    let _ = cli.get("1");
    let _ = cli.query(q);
    let _ = cli.act(create_request);
    let _ = cli.act(comment_request);
    let _ = cli.act_by_id("1", update_request);
    let _ = cli.act_by_id("1", like_request);
    let _ = cli.user_info("wallet".to_string(), "email".to_string(), 1);
    let _ = cli.check_email("email".to_string());
    // let _ = cli.search_by("test".to_string(), 0);
    let _ = cli.create("title".to_string(), "description".to_string());
    let _ = cli.update("id", "description".to_string(), 1, vec!["tag".to_string()]);
    let _ = cli.search_by(
        1,
        Some("bookmark".to_string()),
        "description".to_string(),
        10,
    );
    let _ = cli.date_from(10, None, 20240101);
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
#[cfg_attr(feature = "server", derive(schemars::JsonSchema, aide::OperationIo))]
#[api_model(base = "/users/v1", iter_type=Vec, read_action = user_info, table = users)]
pub struct User {
    #[api_model(primary_key)]
    pub id: String,
    #[api_model(type = TIMESTAMP)]
    pub created_at: u64,
    #[api_model(type = TIMESTAMP)]
    pub updated_at: u64,

    #[api_model(action = signup)]
    pub nickname: String,
    #[api_model(action = signup, read_action = [check_email])]
    pub email: String,
    #[api_model(action = signup)]
    pub profile_url: String,
}

#[test]
fn test_macro_expansion_user() {
    let _ = User {
        id: "id".to_string(),
        created_at: 0,
        updated_at: 0,
        nickname: "nickname".to_string(),
        email: "email".to_string(),
        profile_url: "profile_url".to_string(),
    };

    let _ = UserReadAction {
        action: Some(UserReadActionType::CheckEmail),
        email: Some("email".to_string()),
    };

    let cli = User::get_client("");
    let _ = cli.user_info();
    let _ = cli.check_email("email".to_string());
}

#[test]
fn test_macro_expansion_comment() {
    let cli = Comment::get_client("");
    let _ = cli.get("topic-id", "comment-id");
    let _ = cli.query("topic-id", CommentQuery::new(10));
    let _ = cli.act("topic-id", CommentAction::Comment("content".to_string()));
    let _ = cli.comment("topic-id", "content".to_string());
    let _ = cli.search_by("topic-id", "content".to_string());
    let _ = cli.update("topic-id", "comment-id", 100);
    let _ = cli.act_by_id("topic-id", "comment-id", CommentByIdAction::Update(100));
}

#[cfg(feature = "server")]
#[tokio::test]
async fn test_db_create() {
    use sqlx::{postgres::PgPoolOptions, Postgres};

    let _ = tracing_subscriber::fmt::try_init();
    let pool: sqlx::Pool<Postgres> = PgPoolOptions::new()
        .max_connections(5)
        .connect(
            option_env!("DATABASE_URL")
                .unwrap_or("postgres://postgres:password@localhost:5432/test"),
        )
        .await
        .unwrap();

    let repo = User::get_repository(&pool);
    repo.create_table().await.unwrap();

    let repo = Topic::get_repository(&pool);
    repo.create_table().await.unwrap();

    let repo = Comment::get_repository(&pool);
    repo.create_table().await.unwrap();
}
