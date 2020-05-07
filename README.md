# actix-web-example
 `actix-web-example` is a complete website created using [actix-web](https://actix.rs/). 
 It uses [diesel](https://diesel.rs/) for interacting with the database.
 
 ## How to run
 1. This uses MySql (or Maria DB) so make sure that MySql (or Maria DB) is installed and running.
 Feel free to change the code to use any other database.
 2. Diesel is also required. To install it 
 ```
 cargo install diesel_cli --no-default-features --features mysql
 ```
 3. Change .env to be of the form 
 ```
 DATABASE_URL=mysql://username:password@localhost/actix_web_example
 ```
 4. Run the following commands
 ```
 diesel setup
 diesel migration run
 ```
 
