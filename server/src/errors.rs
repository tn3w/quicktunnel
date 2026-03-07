use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};

pub fn html_escape(input: &str) -> String {
    input
        .chars()
        .flat_map(|c| match c {
            '&' => "&amp;".chars().collect::<Vec<_>>(),
            '<' => "<".chars().collect(),
            '>' => ">".chars().collect(),
            '"' => "\"".chars().collect(),
            '\'' => "&#x27;".chars().collect(),
            c if c.is_ascii_control() && !matches!(c, '\n' | '\t') => vec![],
            c => vec![c],
        })
        .collect()
}

struct ErrorPage<'a> {
    code: &'a str,
    title: &'a str,
    message: &'a str,
    tunnel_status: &'a str,
    local_status: &'a str,
    tunnel_badge: bool,
    local_badge: bool,
    detail: Option<&'a str>,
    developer_steps: Vec<&'a str>,
    visitor_steps: Vec<&'a str>,
    tunnel_domain: &'a str,
}

fn toggle_block(rendered: &mut String, tag: &str, show: bool) {
    let open = format!("{{{{#{}}}}}", tag);
    let close = format!("{{{{/{}}}}}", tag);

    if show {
        *rendered = rendered.replace(&open, "").replace(&close, "");
        return;
    }

    if let (Some(start), Some(end)) = (rendered.find(&open), rendered.find(&close)) {
        rendered.replace_range(start..end + close.len(), "");
    }
}

fn steps_list(steps: Vec<&str>) -> String {
    steps
        .iter()
        .map(|s| format!("<li>{}</li>", html_escape(s)))
        .collect()
}

fn render(page: ErrorPage) -> Response {
    let mut html = std::fs::read_to_string("./dist/error.html").unwrap_or_else(|_| {
        format!(
            "<html><body><h1>{}</h1><p>{}</p></body></html>",
            html_escape(page.code),
            html_escape(page.message)
        )
    });

    let tunnel_line = format!("{}-line", page.tunnel_status);
    let local_line = format!("{}-line", page.local_status);

    html = html
        .replace("{{error_code}}", &html_escape(page.code))
        .replace("{{error_title}}", &html_escape(page.title))
        .replace("{{error_message}}", &html_escape(page.message))
        .replace("{{tunnel_status}}", page.tunnel_status)
        .replace("{{tunnel_line_status}}", &tunnel_line)
        .replace("{{local_status}}", page.local_status)
        .replace("{{local_line_status}}", &local_line)
        .replace("{{tunnel_domain}}", &html_escape(page.tunnel_domain));

    toggle_block(&mut html, "tunnel_error_badge", page.tunnel_badge);
    toggle_block(&mut html, "local_error_badge", page.local_badge);

    match page.detail {
        Some(detail) => {
            toggle_block(&mut html, "technical_detail", true);
            html = html.replace("{{technical_detail}}", &html_escape(detail));
        }
        None => toggle_block(&mut html, "technical_detail", false),
    }

    html = html
        .replace("{{#developer_steps}}", "")
        .replace("{{/developer_steps}}", "")
        .replace("{{developer_steps}}", &steps_list(page.developer_steps))
        .replace("{{#visitor_steps}}", "")
        .replace("{{/visitor_steps}}", "")
        .replace("{{visitor_steps}}", &steps_list(page.visitor_steps));

    let mut headers = HeaderMap::new();
    headers.insert("content-type", "text/html; charset=utf-8".parse().unwrap());

    (StatusCode::BAD_GATEWAY, headers, html).into_response()
}

pub fn tunnel_not_found_error(host: &str, tunnel_domain: &str) -> Response {
    let subdomain = host.split('.').next().unwrap_or("unknown");
    render(ErrorPage {
        code: "404",
        title: "Tunnel Not Found",
        message: "The tunnel you're trying to reach doesn't exist or has been closed.",
        tunnel_status: "error",
        local_status: "error",
        tunnel_badge: true,
        local_badge: true,
        detail: Some(&format!("No active tunnel for subdomain: {}", subdomain)),
        developer_steps: vec![
            "Verify the SSH tunnel is still running on your machine",
            "Check that you're using the correct subdomain from the tunnel banner",
            &format!(
                "Restart the tunnel with: ssh -NR 80:localhost:PORT {}",
                tunnel_domain
            ),
        ],
        visitor_steps: vec![
            "Wait a moment and refresh the page",
            "Contact the developer and ask them to restart their tunnel",
            "Verify you have the correct URL",
        ],
        tunnel_domain,
    })
}

pub fn tunnel_not_connected_error(tunnel_domain: &str) -> Response {
    render(ErrorPage {
        code: "502",
        title: "Tunnel Not Connected",
        message: "The tunnel exists but hasn't established a connection yet.",
        tunnel_status: "warning",
        local_status: "error",
        tunnel_badge: false,
        local_badge: true,
        detail: None,
        developer_steps: vec![
            "Wait a few seconds for the SSH connection to establish",
            "Check your terminal for any SSH connection errors",
            "Ensure your firewall isn't blocking outbound SSH connections",
        ],
        visitor_steps: vec![
            "Wait 10-15 seconds and refresh the page",
            "The tunnel is being established, please be patient",
        ],
        tunnel_domain,
    })
}

pub fn upstream_connection_failed_error(detail: Option<&str>, tunnel_domain: &str) -> Response {
    render(ErrorPage {
        code: "502",
        title: "Connection Failed",
        message: "Traffic reached the tunnel, but couldn't connect to your local service.",
        tunnel_status: "success",
        local_status: "error",
        tunnel_badge: false,
        local_badge: true,
        detail,
        developer_steps: vec![
            "Make sure a service is running on your local machine",
            "Test locally: curl http://localhost:PORT",
            "Check that your application started without errors",
            "Verify the port number matches your running service",
        ],
        visitor_steps: vec![
            "Wait a few minutes and refresh the page",
            "The developer's local service may be starting up",
            "Contact the developer if the issue persists",
        ],
        tunnel_domain,
    })
}

pub fn tunnel_send_failed_error(tunnel_domain: &str) -> Response {
    upstream_connection_failed_error(
        Some("Failed to write data to SSH tunnel channel"),
        tunnel_domain,
    )
}

pub fn request_body_too_large_error(tunnel_domain: &str) -> Response {
    render(ErrorPage {
        code: "413",
        title: "Request Too Large",
        message: "The request body exceeds the maximum allowed size of 10MB.",
        tunnel_status: "success",
        local_status: "error",
        tunnel_badge: false,
        local_badge: true,
        detail: Some("Request body size limit: 10MB"),
        developer_steps: vec![
            "Reduce the size of the data you're sending",
            "If uploading files, compress them or split into smaller chunks",
            "Consider implementing chunked uploads in your application",
            "Check if your application is sending unnecessary data",
        ],
        visitor_steps: vec![
            "The request you sent is too large for this tunnel",
            "Contact the developer about the size limits",
            "Try reducing the amount of data in your request",
        ],
        tunnel_domain,
    })
}

pub fn response_too_large_error(tunnel_domain: &str) -> Response {
    render(ErrorPage {
        code: "502",
        title: "Response Too Large",
        message: "Your local service returned a response larger than 50MB, which exceeds the tunnel limit.",
        tunnel_status: "success",
        local_status: "error",
        tunnel_badge: false,
        local_badge: true,
        detail: Some("Response size limit: 50MB"),
        developer_steps: vec![
            "Reduce the size of the response your service is returning",
            "Implement pagination for large datasets",
            "Compress response data before sending",
            "Consider serving large files through a CDN instead",
        ],
        visitor_steps: vec![
            "The response from the service is too large",
            "Contact the developer about this limitation",
            "Try requesting less data if possible",
        ],
        tunnel_domain,
    })
}

pub fn tunnel_timeout_error(tunnel_domain: &str) -> Response {
    render(ErrorPage {
        code: "504",
        title: "Gateway Timeout",
        message: "Your local service didn't respond within 30 seconds.",
        tunnel_status: "success",
        local_status: "error",
        tunnel_badge: false,
        local_badge: true,
        detail: Some("Timeout after 30 seconds waiting for response"),
        developer_steps: vec![
            "Check if your local service is running and responsive",
            "Look for slow database queries or external API calls",
            "Optimize your application's response time",
            "Check your application logs for errors or hangs",
        ],
        visitor_steps: vec![
            "Wait a moment and try again",
            "The service may be processing a slow request",
            "Contact the developer if timeouts persist",
        ],
        tunnel_domain,
    })
}

fn service_bug_error(
    title: &str,
    message: &str,
    detail: &'static str,
    tunnel_domain: &str,
) -> Response {
    render(ErrorPage {
        code: "502",
        title,
        message,
        tunnel_status: "success",
        local_status: "error",
        tunnel_badge: false,
        local_badge: true,
        detail: Some(detail),
        developer_steps: vec![
            "Verify your service is sending valid HTTP responses",
            "Test your service locally to verify HTTP compliance",
            "Check your application logs for errors",
            "Report this issue if it persists",
        ],
        visitor_steps: vec![
            "The service returned an invalid response",
            "Contact the developer about this error",
            "This is likely a bug in the application",
        ],
        tunnel_domain,
    })
}

pub fn empty_response_error(tunnel_domain: &str) -> Response {
    service_bug_error(
        "Empty Response",
        "Your local service closed the connection without sending any data.",
        "No data received from local service",
        tunnel_domain,
    )
}

pub fn malformed_response_error(tunnel_domain: &str) -> Response {
    service_bug_error(
        "Malformed Response",
        "Your local service returned an invalid HTTP response.",
        "HTTP response missing required headers separator",
        tunnel_domain,
    )
}

pub fn invalid_headers_error(tunnel_domain: &str) -> Response {
    service_bug_error(
        "Invalid Response Headers",
        "Your local service returned HTTP headers with invalid characters.",
        "Response headers contain non-UTF8 characters",
        tunnel_domain,
    )
}

pub fn chunked_decode_error(tunnel_domain: &str) -> Response {
    service_bug_error(
        "Chunked Encoding Error",
        "Your local service sent a response with invalid chunked transfer encoding.",
        "Failed to decode chunked transfer encoding",
        tunnel_domain,
    )
}

pub fn response_construction_error(tunnel_domain: &str) -> Response {
    service_bug_error(
        "Response Construction Failed",
        "Failed to construct the HTTP response from your local service's data.",
        "Internal error building HTTP response",
        tunnel_domain,
    )
}
