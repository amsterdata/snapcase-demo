use duckdb::Connection;
use duckdb::Row;

pub struct PurchaseDatabase {
    duckdb: Connection,
}

type BasketId = usize;
type ItemId = usize;

#[derive(Debug, Serialize, Deserialize)]
pub struct Purchases {
    pub user_id: usize,
    pub item_purchases: Vec<ItemPurchase>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ItemPurchase {
    pub day_of_week: usize,
    pub hour_of_day: usize,
    pub basket_id: usize,
    pub item_id: usize,
}


impl PurchaseDatabase {
    pub fn new() -> Self {
        let duckdb = Connection::open_in_memory().unwrap();
        eprintln!("[DuckDB] Importing purchase database for 206,209 users, 49,685 items and 33,819,106 purchases...");
        for table in ["aisles", "departments", "order_products", "orders", "products"] {
            let import_query = format!(r#"
                CREATE TABLE {table} AS
                SELECT * FROM read_parquet('datasets/instacart/{table}.parquet');
            "#);
            duckdb.execute(&import_query, []).unwrap();
            eprintln!("[DuckDB] \tImported table {table}.");
        }
        Self { duckdb }
    }

    pub fn execute(&self, query: &str) {
        self.duckdb.execute(&query, []).unwrap();
    }

    pub fn from_query<F>(&self, query: &str, mut consumer: F)
        where
            F: FnMut(&Row<'_>) -> ()
    {
        let mut stmt = self.duckdb.prepare(query).unwrap();
        let mut rows = stmt.query([]).unwrap();
        while let Some(row) = rows.next().unwrap() {
            consumer(row);
        }
    }

    pub fn purchases(&self, user_id: usize) -> Purchases {
        let mut item_purchases = Vec::new();
        self.from_query(&format!(r#"
                SELECT    o.order_dow, o.order_hour_of_day, o.order_id, op.product_id
                  FROM    products p
                  JOIN    order_products op
                    ON    p.product_id = op.product_id
                  JOIN    orders o
                    ON    o.order_id = op.order_id
                 WHERE    o.user_id = {user_id}
              ORDER BY    op.order_id;
                "#, ),
            |row| {
                let day_of_week: usize = row.get(0).unwrap();
                let hour_of_day: usize = row.get(1).unwrap();
                let basket_id: BasketId = row.get(2).unwrap();
                let item_id: ItemId = row.get(3).unwrap();
                item_purchases.push(ItemPurchase{
                    day_of_week,
                    hour_of_day,
                    basket_id,
                    item_id
                })
            }
        );

        Purchases { user_id, item_purchases }
    }

    pub fn alcohol_purchases<F>(&self, user_id: usize, mut consumer: F)
        where
            F: FnMut((BasketId, ItemId)) -> ()
    {
        self.from_query(&format!(r#"
                SELECT    op.order_id, op.product_id
                  FROM    'datasets/instacart/products.parquet' p
                  JOIN    'datasets/instacart/order_products.parquet' op
                    ON    p.product_id = op.product_id
                  JOIN    'datasets/instacart/orders.parquet' o
                    ON    o.order_id = op.order_id
                 WHERE    p.aisle_id IN (27, 28, 62, 124, 134)
                   AND    o.user_id = {user_id};
                "#,),
                        |row| {
                            let basket_id: BasketId = row.get(0).unwrap();
                            let item_id: ItemId = row.get(1).unwrap();
                            consumer((basket_id, item_id));
                        }
        );
    }


}