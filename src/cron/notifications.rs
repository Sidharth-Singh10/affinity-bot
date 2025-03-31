use ::entity::{notification_preferences, price_history, products};
use async_trait::async_trait;
use chrono::Utc;
use prelude::Decimal;
use sea_orm::*;
use tokio::time::{interval, Duration};

use crate::utils::emails::notify_price::PriceHistoryEmail;

// Trait for notification preferences
#[async_trait]
pub trait NotificationPreference {
    async fn should_notify(&self, db: &DatabaseConnection) -> Result<bool, DbErr>;
    async fn send_notification(&self, db: &DatabaseConnection) -> Result<(), DbErr>;
    async fn update_last_notified(&self, db: &DatabaseConnection) -> Result<(), DbErr>;
}

impl From<notification_preferences::Model> for MyntraNotification {
    fn from(model: notification_preferences::Model) -> Self {
        Self {
            preference_id: model.preference_id,
            product_id: model.product_id,
            email: model.email,
            time_interval: model.time_interval_hours,
            price_threshold: model.price_threshold,
            notify_on_lowest: model.notify_on_lowest,
            notify_on_highest: model.notify_on_highest,
            last_notified: chrono::DateTime::from_naive_utc_and_offset(model.last_notified, Utc),
        }
    }
}

// Scope for improvement. Dead Code warning here , need to remove this
pub struct MyntraNotification {
    preference_id: i32,
    product_id: i32,
    email: String,
    time_interval: i32,
    price_threshold: Decimal,
    notify_on_lowest: bool,
    notify_on_highest: bool,
    last_notified: chrono::DateTime<Utc>,
}

#[async_trait]
impl NotificationPreference for MyntraNotification {
    // Scope of improvement
    async fn should_notify(&self, _: &DatabaseConnection) -> Result<bool, DbErr> {
        if Utc::now() - self.last_notified > chrono::Duration::hours(self.time_interval.into()) {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn send_notification(&self, db: &DatabaseConnection) -> Result<(), DbErr> {
        let existing_product = products::Entity::find()
            .filter(products::Column::ProductId.eq(self.product_id))
            .one(db)
            .await?;

        let price_history = price_history::Entity::find()
            .filter(price_history::Column::ProductId.eq(self.product_id))
            .order_by(price_history::Column::RecordedAt, Order::Desc)
            .all(db)
            .await?;

        if let Some(product) = existing_product {
            let prices: Vec<(prelude::DateTime, Decimal)> = price_history
                .iter()
                .map(|ph| (ph.recorded_at, ph.price))
                .collect();

            let current_price = prices.first().map(|(_, p)| *p).unwrap_or_default();
            let highest_price = *prices
                .iter()
                .map(|(_, p)| p)
                .max()
                .unwrap_or(&current_price);
            let lowest_price = *prices
                .iter()
                .map(|(_, p)| p)
                .min()
                .unwrap_or(&current_price);

            let email = PriceHistoryEmail::new(
                product.product_id.to_string(), // Scope for Improvement
                current_price,
                highest_price,
                lowest_price,
                prices,
                self.email.clone(),
            );

            email
                .send_price_history()
                .await
                .map_err(DbErr::Custom)?;
        }

        Ok(())
    }

    async fn update_last_notified(&self, db: &DatabaseConnection) -> Result<(), DbErr> {
        notification_preferences::Entity::update_many()
            .filter(notification_preferences::Column::ProductId.eq(self.product_id))
            .set(notification_preferences::ActiveModel {
                last_notified: Set(Utc::now().naive_utc()),
                ..Default::default()
            })
            .exec(db)
            .await?;
        Ok(())
    }
}

// Notification manager
pub struct NotificationManager {
    db: DatabaseConnection,
    handlers: Vec<Box<dyn NotificationHandler + Send + Sync>>,
}

#[async_trait]
pub trait NotificationHandler: Send + Sync {
    async fn check_notifications(&self, db: &DatabaseConnection) -> Result<(), DbErr>;
}

// Handler implementations
pub struct MyntraHandler;

#[async_trait]
impl NotificationHandler for MyntraHandler {
    async fn check_notifications(&self, db: &DatabaseConnection) -> Result<(), DbErr> {
        let preferences = notification_preferences::Entity::find().all(db).await?;

        for pref in preferences {
            let notification = MyntraNotification::from(pref);
            if notification.should_notify(db).await? && notification.send_notification(db).await.is_ok() {
                notification.update_last_notified(db).await?;
            }
        }
        Ok(())
    }
}

impl NotificationManager {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            handlers: Vec::new(),
        }
    }

    pub fn register_handler<H: NotificationHandler + 'static>(&mut self, handler: H) {
        self.handlers.push(Box::new(handler));
    }

    pub async fn start(self) {
        let mut interval = interval(Duration::from_secs(3600));

        tokio::spawn(async move {
            loop {
                interval.tick().await;
                for handler in &self.handlers {
                    if let Err(e) = handler.check_notifications(&self.db).await {
                        eprintln!("Error checking notifications: {}", e);
                    }
                }
            }
        });
    }
}
