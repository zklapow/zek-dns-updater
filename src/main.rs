use std::env;
use std::process::exit;

use cloudflare::endpoints::{dns, zone};
use cloudflare::framework::{
    apiclient::ApiClient,
    auth::Credentials,
    mock::{MockApiClient, NoopEndpoint},
    response::{ApiFailure, ApiResponse, ApiResult},
    Environment, HttpApiClient, HttpApiClientConfig, OrderDirection,
};
use std::net::Ipv6Addr;

fn main() {
    let skip_dns = env::var("SKIP_DNS").ok().unwrap_or("false".to_owned());
    if skip_dns.to_lowercase() == "true" {
        println!("Skipping DNS update!");
        exit(0);
    }

    let fqdns_var = env::var("FQDNS").unwrap_or("".to_owned());
    let fqdns: Vec<&str> = fqdns_var.split(",").collect();
    if fqdns.len() == 0 {
        println!("No domains to udpate");
        exit(0);
    }

    let addr: Ipv6Addr = env::var("IPV6_ADDR").expect("No IPV6_ADDR var set").parse().expect("Could no parse IP address");
    let zone_id = env::var("DNS_ZONE_ID").expect("No DNS_ZONE_ID var set");

    let cf_email = env::var("CF_API_EMAIL")
        .expect("Attempted to update cloudlflare domains without a CF_API_EMAIL set");
    let cf_key = env::var("CF_API_TOKEN")
        .expect("Attempted to update cloudlflare domains without a CF_API_TOKEN set");

    let creds = Credentials::UserAuthKey {
        email: cf_email,
        key: cf_key
    };

    let cf_client = HttpApiClient::new(
        creds, HttpApiClientConfig::default(), Environment::Production
    ).ok().expect("Could not create API client");

    let args: Vec<String> = env::args().collect();

    let default_command: String = "create".to_owned();
    let command = args.get(1).unwrap_or(&default_command);

    if command.to_lowercase() == "create" {
        println!("Creating {} records pointing to {}", fqdns.len(), addr);
        create_records(cf_client, zone_id, addr, fqdns)
    } else if command.to_lowercase() == "delete" {
        println!("Deleting {} records pointing to {}", fqdns.len(), addr);
        delete_records(cf_client, zone_id, fqdns);
    }
}

fn delete_records(cf_client: HttpApiClient, zone_id: String, fqdns: Vec<&str>) {
    let list_req = dns::ListDnsRecords {
        zone_identifier: zone_id.as_str(),
        params: dns::ListDnsRecordsParams {
            record_type: None,
            name: None,
            page: None,
            per_page: Some(1000),
            order: None,
            direction: None,
            search_match: None,
        }
    };

    let records: Vec<cloudflare::endpoints::dns::DnsRecord> = cf_client.request(&list_req).expect("Cannot list records to delete").result;
    for record in records {
        if fqdns.contains(&record.name.as_str()) {
            println!("Deleting {}", record.name);

            let delete_req = dns::DeleteDnsRecord {
                zone_identifier: zone_id.as_str(),
                identifier: record.id.as_str(),
            };

            cf_client.request(&delete_req);
        }
    }
}

fn create_records(cf_client: HttpApiClient, zone_id: String, addr: Ipv6Addr, fqdns: Vec<&str>) {
    for subdomain in fqdns {
        println!("Updating {}", subdomain);

        // TODO: Don't hardcode
        let req = dns::CreateDnsRecord {
            zone_identifier: zone_id.as_str(),
            params: dns::CreateDnsRecordParams {
                name: subdomain,
                content: dns::DnsContent::AAAA { content: addr },
                ttl: None,
                priority: None,
                proxied: None,
            }
        };
        cf_client.request(&req).expect("Failed to update domain");
    }
}
