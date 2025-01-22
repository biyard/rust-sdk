use easy_dynamodb::error::DynamoException;

#[derive(Debug)]
pub enum DatabaseError {
    DynamoDbError(DynamoException),
    SqlError(sqlx::Error),
}

impl From<DynamoException> for DatabaseError {
    fn from(e: DynamoException) -> Self {
        DatabaseError::DynamoDbError(e)
    }
}

impl From<sqlx::Error> for DatabaseError {
    fn from(e: sqlx::Error) -> Self {
        DatabaseError::SqlError(e)
    }
}
