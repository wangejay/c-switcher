use c_switcher_lib::{
    get_usage_impl, list_profiles_impl, switch_profile_impl, ProfileEntry, UsageResult,
};
use std::os::unix::process::CommandExt;
use std::process::Command;
use tokio::task::JoinSet;

// ── ANSI color codes ──

const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const CYAN: &str = "\x1b[36m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

// ── CLI argument parsing ──

enum Mode {
    Auto,
    List,
    Interactive,
}

struct CliArgs {
    mode: Mode,
    claude_args: Vec<String>,
}

fn parse_args() -> CliArgs {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut mode = Mode::Auto;
    let mut claude_args = Vec::new();
    let mut after_dashdash = false;

    for arg in &args {
        if after_dashdash {
            claude_args.push(arg.clone());
            continue;
        }
        if arg == "--" {
            after_dashdash = true;
            continue;
        }
        match arg.as_str() {
            "-l" | "--list" => mode = Mode::List,
            "-i" | "--interactive" => mode = Mode::Interactive,
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            other => {
                eprintln!("{}csw: unknown option: {}{}", RED, other, RESET);
                print_help();
                std::process::exit(1);
            }
        }
    }

    CliArgs { mode, claude_args }
}

fn print_help() {
    eprintln!(
        "{}csw{} — Claude Code account switcher\n",
        BOLD, RESET
    );
    eprintln!("{}USAGE:{}", BOLD, RESET);
    eprintln!("  csw                        Auto: pick lowest usage → switch → exec claude");
    eprintln!("  csw -l                     List: print usage table, don't switch");
    eprintln!("  csw -i                     Interactive: list → pick → switch → exec claude");
    eprintln!("  csw -- --skip-permissions  Auto + pass args to claude");
    eprintln!("  csw -i -- --skip-perms     Interactive + pass args to claude");
    eprintln!();
    eprintln!("{}OPTIONS:{}", BOLD, RESET);
    eprintln!("  -l, --list           Print usage table and exit");
    eprintln!("  -i, --interactive    Interactive profile selection");
    eprintln!("  -h, --help           Show this help");
    eprintln!("  --                   Pass remaining args to claude");
}

// ── Usage data ──

struct ProfileUsage {
    name: String,
    email: String,
    organization: String,
    five_hour_pct: f64,
    seven_day_pct: f64,
    monthly_pct: f64,
    error: Option<String>,
}

async fn fetch_all_usage(profiles: &[ProfileEntry]) -> Vec<ProfileUsage> {
    let mut set = JoinSet::new();

    for p in profiles {
        let name = p.name.clone();
        let email = p.email.clone();
        let organization = p.organization.clone();
        set.spawn(async move {
            let result = get_usage_impl(Some(name.clone())).await;
            parse_usage_result(&name, &email, &organization, result)
        });
    }

    let mut usages = Vec::with_capacity(profiles.len());
    while let Some(result) = set.join_next().await {
        match result {
            Ok(pu) => usages.push(pu),
            Err(e) => {
                usages.push(ProfileUsage {
                    name: "?".into(),
                    email: "?".into(),
                    organization: String::new(),
                    five_hour_pct: 999.0,
                    seven_day_pct: 999.0,
                    monthly_pct: 999.0,
                    error: Some(format!("join error: {}", e)),
                });
            }
        }
    }

    // Sort: five_hour_pct ascending, tiebreak monthly_pct ascending
    usages.sort_by(|a, b| {
        a.five_hour_pct
            .partial_cmp(&b.five_hour_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(
                a.monthly_pct
                    .partial_cmp(&b.monthly_pct)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
    });

    usages
}

fn parse_usage_result(
    name: &str,
    email: &str,
    organization: &str,
    result: UsageResult,
) -> ProfileUsage {
    if !result.success {
        return ProfileUsage {
            name: name.to_string(),
            email: email.to_string(),
            organization: organization.to_string(),
            five_hour_pct: 999.0,
            seven_day_pct: 999.0,
            monthly_pct: 999.0,
            error: result.error,
        };
    }

    let data = match result.data {
        Some(d) => d,
        None => {
            return ProfileUsage {
                name: name.to_string(),
                email: email.to_string(),
                organization: organization.to_string(),
                five_hour_pct: 999.0,
                seven_day_pct: 999.0,
                monthly_pct: 999.0,
                error: Some("no data".into()),
            };
        }
    };

    let five_hour_pct = data
        .get("five_hour")
        .and_then(|v| v.get("utilization"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let seven_day_pct = data
        .get("seven_day")
        .and_then(|v| v.get("utilization"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let monthly_pct = data
        .get("utilization")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    ProfileUsage {
        name: name.to_string(),
        email: email.to_string(),
        organization: organization.to_string(),
        five_hour_pct,
        seven_day_pct,
        monthly_pct,
        error: None,
    }
}

fn pick_lowest_usage(usages: &[ProfileUsage]) -> Option<&ProfileUsage> {
    usages.iter().find(|u| u.error.is_none()).or(usages.first())
}

fn color_for_pct(pct: f64) -> &'static str {
    if pct < 50.0 {
        GREEN
    } else if pct < 80.0 {
        YELLOW
    } else {
        RED
    }
}

fn print_usage_table(usages: &[ProfileUsage]) {
    eprintln!(
        "\n{}{}  {:>2}  {:<16}  {:<30}  {:<18}  {:>7}  {:>7}{}",
        BOLD, DIM, "#", "Profile", "Email", "Org", "5h%", "7d%", RESET
    );
    eprintln!(
        "{}  {:─>2}  {:─>16}  {:─>30}  {:─>18}  {:─>7}  {:─>7}{}",
        DIM, "", "", "", "", "", "", RESET
    );

    for (i, u) in usages.iter().enumerate() {
        let idx = i + 1;
        let name = truncate_str(&u.name, 16);
        let email = truncate_str(&u.email, 30);
        let org = truncate_str(&u.organization, 18);

        if let Some(ref err) = u.error {
            eprintln!(
                "  {}{:>2}{}  {:<16}  {:<30}  {:<18}  {}ERROR: {}{}",
                DIM, idx, RESET, name, email, org, RED, truncate_str(err, 30), RESET
            );
        } else {
            let c5 = color_for_pct(u.five_hour_pct);
            let c7 = color_for_pct(u.seven_day_pct);
            eprintln!(
                "  {}{:>2}{}  {:<16}  {:<30}  {:<18}  {}{:>6.1}%{}  {}{:>6.1}%{}",
                DIM, idx, RESET,
                name, email, org,
                c5, u.five_hour_pct, RESET,
                c7, u.seven_day_pct, RESET,
            );
        }
    }
    eprintln!();
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max - 1).collect();
        format!("{}…", truncated)
    }
}

fn prompt_user_choice(count: usize) -> usize {
    eprint!("{}Select profile [1-{}]: {}", CYAN, count, RESET);

    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .expect("Failed to read stdin");

    match input.trim().parse::<usize>() {
        Ok(n) if n >= 1 && n <= count => n,
        _ => {
            eprintln!("{}Invalid choice.{}", RED, RESET);
            std::process::exit(1);
        }
    }
}

async fn switch_and_exec(profile_name: &str, claude_args: &[String]) -> ! {
    eprintln!(
        "{}Switching to profile: {}{}{}",
        DIM, RESET, BOLD, profile_name
    );

    let result = switch_profile_impl(profile_name.to_string()).await;
    if !result.success {
        let err = result.error.unwrap_or_else(|| "unknown error".into());
        eprintln!("{}Switch failed: {}{}", RED, err, RESET);
        std::process::exit(1);
    }

    if let (Some(from), Some(to)) = (&result.from, &result.to) {
        eprintln!("{}Switched: {} → {}{}", GREEN, from, to, RESET);
    }

    eprintln!("{}Launching claude...{}", DIM, RESET);

    let err = Command::new("claude").args(claude_args).exec();
    // exec() only returns on error
    eprintln!("{}Failed to exec claude: {}{}", RED, err, RESET);
    std::process::exit(1);
}

#[tokio::main]
async fn main() {
    let args = parse_args();

    // Fetch profiles
    let profiles = list_profiles_impl().await;
    if profiles.is_empty() {
        eprintln!(
            "{}No profiles found. Use the Tauri app to backup profiles first.{}",
            RED, RESET
        );
        std::process::exit(1);
    }

    // Fetch usage for all profiles in parallel
    eprintln!("{}Fetching usage for {} profiles...{}", DIM, profiles.len(), RESET);
    let usages = fetch_all_usage(&profiles).await;

    match args.mode {
        Mode::List => {
            print_usage_table(&usages);
        }
        Mode::Auto => {
            print_usage_table(&usages);
            match pick_lowest_usage(&usages) {
                Some(best) => {
                    switch_and_exec(&best.name, &args.claude_args).await;
                }
                None => {
                    eprintln!("{}No usable profile found.{}", RED, RESET);
                    std::process::exit(1);
                }
            }
        }
        Mode::Interactive => {
            print_usage_table(&usages);
            let choice = prompt_user_choice(usages.len());
            let selected = &usages[choice - 1];
            switch_and_exec(&selected.name, &args.claude_args).await;
        }
    }
}
