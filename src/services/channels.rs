use sqlx::{PgPool, Postgres, QueryBuilder};

use crate::config::API_PREFIX;
use crate::models::{Channel, CustomChannel, IptvChannel};
use crate::schemas::{CategoryOut, ChannelOut};

pub fn to_out(row: &Channel) -> ChannelOut {
    ChannelOut {
        id: row.id.clone(),
        uuid: row.uuid.clone(),
        number: row.number,
        name: row.name.clone(),
        category: row.category.clone(),
        initials: row.initials.clone(),
        accent: row.accent.clone(),
        logo_url: format!("/static/logos/{}", row.logo_file),
        description: row.description.clone(),
        stream_url: row.stream_url.clone(),
        website: row.website.clone(),
        source: None,
    }
}

pub fn to_iptv_out(row: &IptvChannel) -> ChannelOut {
    let logo = if row.logo_url.is_some() {
        format!("{}/iptv/channels/{}/logo", *API_PREFIX, row.id)
    } else {
        String::new()
    };
    ChannelOut {
        id: row.id.clone(),
        uuid: row.uuid.clone(),
        number: row.number,
        name: row.name.clone(),
        category: row.category.clone(),
        initials: row.initials.clone(),
        accent: row.accent.clone(),
        logo_url: logo,
        description: None,
        stream_url: row.stream_url.clone(),
        website: row.website.clone(),
        source: Some("iptv".into()),
    }
}

pub fn to_custom_out(row: &CustomChannel) -> ChannelOut {
    let logo = if row.logo_url.is_some() {
        format!("{}/custom/channels/{}/logo", *API_PREFIX, row.id)
    } else {
        String::new()
    };
    ChannelOut {
        id: row.id.clone(),
        uuid: row.uuid.clone(),
        number: row.number,
        name: row.name.clone(),
        category: row.category.clone(),
        initials: row.initials.clone(),
        accent: row.accent.clone(),
        logo_url: logo,
        description: row.group_name.clone(),
        stream_url: Some(row.url.clone()),
        website: None,
        source: Some("custom".into()),
    }
}

fn push_filters(
    qb: &mut QueryBuilder<'_, Postgres>,
    category: Option<&str>,
    q: Option<&str>,
    with_description: bool,
) {
    if let Some(cat) = category {
        if !cat.is_empty() {
            qb.push(" AND lower(category) = ");
            qb.push_bind(cat.trim().to_lowercase());
        }
    }
    if let Some(q) = q {
        if !q.is_empty() {
            let pattern = format!("%{}%", q.trim());
            qb.push(" AND (name ILIKE ");
            qb.push_bind(pattern.clone());
            qb.push(" OR category ILIKE ");
            qb.push_bind(pattern.clone());
            if with_description {
                qb.push(" OR description ILIKE ");
                qb.push_bind(pattern);
            }
            qb.push(")");
        }
    }
}

pub async fn list_channels(
    pool: &PgPool,
    category: Option<&str>,
    q: Option<&str>,
) -> Result<Vec<ChannelOut>, sqlx::Error> {
    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new("SELECT * FROM channels WHERE 1=1");
    push_filters(&mut qb, category, q, true);
    qb.push(" ORDER BY number");
    let rows = qb.build_query_as::<Channel>().fetch_all(pool).await?;

    let mut qb: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT * FROM custom_channels WHERE 1=1");
    push_filters(&mut qb, category, q, false);
    qb.push(" ORDER BY number");
    let customs = qb.build_query_as::<CustomChannel>().fetch_all(pool).await?;

    let mut out: Vec<ChannelOut> = rows.iter().map(to_out).collect();
    out.extend(customs.iter().map(to_custom_out));
    Ok(out)
}

pub async fn get_channel(
    pool: &PgPool,
    channel_id: &str,
) -> Result<Option<ChannelOut>, sqlx::Error> {
    if let Some(row) = sqlx::query_as::<_, Channel>("SELECT * FROM channels WHERE id = $1")
        .bind(channel_id)
        .fetch_optional(pool)
        .await?
    {
        return Ok(Some(to_out(&row)));
    }
    if let Some(row) = sqlx::query_as::<_, IptvChannel>("SELECT * FROM iptv_channels WHERE id = $1")
        .bind(channel_id)
        .fetch_optional(pool)
        .await?
    {
        return Ok(Some(to_iptv_out(&row)));
    }
    if let Some(row) =
        sqlx::query_as::<_, CustomChannel>("SELECT * FROM custom_channels WHERE id = $1")
            .bind(channel_id)
            .fetch_optional(pool)
            .await?
    {
        return Ok(Some(to_custom_out(&row)));
    }
    Ok(None)
}

pub async fn get_channel_by_uuid(
    pool: &PgPool,
    channel_uuid: &str,
) -> Result<Option<ChannelOut>, sqlx::Error> {
    if let Some(row) = sqlx::query_as::<_, Channel>("SELECT * FROM channels WHERE uuid = $1")
        .bind(channel_uuid)
        .fetch_optional(pool)
        .await?
    {
        return Ok(Some(to_out(&row)));
    }
    if let Some(row) =
        sqlx::query_as::<_, IptvChannel>("SELECT * FROM iptv_channels WHERE uuid = $1")
            .bind(channel_uuid)
            .fetch_optional(pool)
            .await?
    {
        return Ok(Some(to_iptv_out(&row)));
    }
    if let Some(row) =
        sqlx::query_as::<_, CustomChannel>("SELECT * FROM custom_channels WHERE uuid = $1")
            .bind(channel_uuid)
            .fetch_optional(pool)
            .await?
    {
        return Ok(Some(to_custom_out(&row)));
    }
    Ok(None)
}

pub async fn update_stream(
    pool: &PgPool,
    channel_id: &str,
    stream_url: Option<String>,
) -> Result<Option<ChannelOut>, sqlx::Error> {
    let exists: Option<String> = sqlx::query_scalar("SELECT id FROM channels WHERE id = $1")
        .bind(channel_id)
        .fetch_optional(pool)
        .await?;
    if exists.is_none() {
        return Ok(None);
    }
    let value = stream_url.filter(|s| !s.is_empty());
    sqlx::query("UPDATE channels SET stream_url = $1 WHERE id = $2")
        .bind(&value)
        .bind(channel_id)
        .execute(pool)
        .await?;
    let row = sqlx::query_as::<_, Channel>("SELECT * FROM channels WHERE id = $1")
        .bind(channel_id)
        .fetch_one(pool)
        .await?;
    Ok(Some(to_out(&row)))
}

pub async fn list_categories(pool: &PgPool) -> Result<Vec<CategoryOut>, sqlx::Error> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT category, count(id) FROM channels GROUP BY category ORDER BY count(id) DESC",
    )
    .fetch_all(pool)
    .await?;

    let mut merged: Vec<(String, i64)> = rows;
    let customs: Vec<(String, i64)> =
        sqlx::query_as("SELECT category, count(id) FROM custom_channels GROUP BY category")
            .fetch_all(pool)
            .await?;
    for (name, count) in customs {
        match merged.iter_mut().find(|(n, _)| *n == name) {
            Some(entry) => entry.1 += count,
            None => merged.push((name, count)),
        }
    }
    merged.sort_by_key(|b| std::cmp::Reverse(b.1));
    Ok(merged
        .into_iter()
        .map(|(name, count)| CategoryOut { name, count })
        .collect())
}
