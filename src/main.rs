fn main() {
    println!("Content-Type: text/html");
    println!();
    println!("<p>Hello, world!</p>");
    println!("<ul>");
    for (key, value) in ::std::env::vars() {
        println!("<li>{}: {}</li>", key, value);
    }
    println!("</ul>");

    ::std::process::exit(0);
}
