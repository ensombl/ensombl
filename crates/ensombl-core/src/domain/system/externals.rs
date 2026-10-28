//! Outside systems recognized by the package a client comes from.

/// Package (or prefix ending in `/`) to the system it talks to.
const SYSTEMS: &[(&str, &str)] = &[
    ("@prisma/client", "Database"),
    ("prisma", "Database"),
    ("typeorm", "Database"),
    ("@nestjs/typeorm", "Database"),
    ("mongoose", "Database"),
    ("@nestjs/mongoose", "Database"),
    ("sequelize", "Database"),
    ("sequelize-typescript", "Database"),
    ("knex", "Database"),
    ("pg", "Database"),
    ("mysql2", "Database"),
    ("mongodb", "Database"),
    ("drizzle-orm", "Database"),
    ("@mikro-orm/", "Database"),
    ("kysely", "Database"),
    ("ioredis", "Redis"),
    ("redis", "Redis"),
    ("cache-manager", "Cache"),
    ("@nestjs/cache-manager", "Cache"),
    ("@aws-sdk/client-s3", "S3"),
    ("@aws-sdk/client-sqs", "SQS"),
    ("@aws-sdk/client-sns", "SNS"),
    ("@aws-sdk/client-ses", "Email"),
    ("@aws-sdk/client-dynamodb", "DynamoDB"),
    ("@aws-sdk/lib-dynamodb", "DynamoDB"),
    ("@aws-sdk/", "AWS"),
    ("aws-sdk", "AWS"),
    ("nodemailer", "Email"),
    ("@sendgrid/mail", "Email"),
    ("resend", "Email"),
    ("bullmq", "Queue"),
    ("bull", "Queue"),
    ("@nestjs/bull", "Queue"),
    ("@nestjs/bullmq", "Queue"),
    ("amqplib", "Message broker"),
    ("kafkajs", "Kafka"),
    ("stripe", "Stripe"),
    ("twilio", "Twilio"),
    ("openai", "OpenAI"),
    ("@anthropic-ai/sdk", "Anthropic"),
    ("firebase-admin", "Firebase"),
    ("firebase", "Firebase"),
    ("@supabase/supabase-js", "Supabase"),
    ("googleapis", "Google APIs"),
    ("@slack/", "Slack"),
    ("@sentry/", "Sentry"),
    ("elasticsearch", "Elasticsearch"),
    ("@elastic/elasticsearch", "Elasticsearch"),
];

/// HTTP client packages; calls through them are matched against routes.
const HTTP_CLIENTS: &[&str] = &[
    "axios",
    "@nestjs/axios",
    "node-fetch",
    "got",
    "ky",
    "undici",
];

pub fn system_for(specifier: &str) -> Option<&'static str> {
    SYSTEMS
        .iter()
        .find(|(pkg, _)| {
            if pkg.ends_with('/') {
                specifier.starts_with(pkg)
            } else {
                specifier == *pkg || specifier.starts_with(&format!("{pkg}/"))
            }
        })
        .map(|(_, system)| *system)
}

pub fn is_http_client(specifier: &str) -> bool {
    HTTP_CLIENTS.contains(&specifier)
}

pub const HTTP_VERBS: &[&str] = &["get", "post", "put", "patch", "delete", "head", "options"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_systems_by_package() {
        assert_eq!(system_for("@prisma/client"), Some("Database"));
        assert_eq!(system_for("@aws-sdk/client-s3"), Some("S3"));
        assert_eq!(system_for("@aws-sdk/client-lambda"), Some("AWS"));
        assert_eq!(system_for("typeorm/repository"), Some("Database"));
        assert_eq!(system_for("prismatic"), None);
        assert!(is_http_client("axios"));
    }
}
