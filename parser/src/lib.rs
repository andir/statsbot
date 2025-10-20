use nom::character::complete::u64;
use serde::Serialize;

use nom::error::ParseError;
use nom::{IResult, Parser, bytes::complete::tag, sequence::delimited};
use simple_prometheus::SimplePrometheus;

#[derive(Debug, Serialize, Default, SimplePrometheus)]
pub struct StatsZ {
    //"Users 10175(1221000) Invites 0(0)"
    users: usize,
    users_memory: usize,
    users_invited: usize,
    users_invited_memory: usize,

    //"User channels 30651(735624) Aways 496(14282)"
    user_channels: usize,
    user_channels_memory: usize,
    users_away: usize,
    users_away_memory: usize,

    // Attached confs 24(576)
    local_client_conf_count: usize,
    local_client_conf_memory: usize,

    //"Conflines 0(0)",
    conf_count: usize,
    conf_memory: usize,

    // "Classes 12(960)",
    classes_count: usize,
    classes_memory: usize,

    // Channels 1988(816734)"
    channels_count: usize,
    channels_memory: usize,

    // "Bans 826(66080) Exceptions 31(2480) Invex 552(44160) Quiets 131(10480)"
    channel_ban_count: usize,
    channel_ban_memory: usize,
    channel_exceptions_count: usize,
    channel_exceptions_memory: usize,
    channel_invex_count: usize,
    channel_invex_memory: usize,
    channel_quiets_count: usize,
    channel_quiets_memory: usize,

    // "Channel members 30651(735624) invite 0(0)",
    channel_members: usize,
    channel_members_memory: usize,
    channel_invites_count: usize,
    channel_invites_memory: usize,

    // "Whowas array 15000(5756672)"
    whowas_count: usize,
    whowas_memory: usize,

    // "Hash: client 131072(3145728) chan 65536(1572864)"
    hash_client_count: usize,
    hash_client_memory: usize,
    hash_channel_count: usize,
    hash_channel_memory: usize,

    // "linebuf 0(0)",
    linebuf_count: usize,
    linebuf_memory: usize,

    // "scache 8(1152)",
    servers_cached_number: usize,
    servers_cached_memory: usize,

    //"hostname hash 131072(3145728)"
    hostname_count: usize,
    hostname_memory: usize,

    // "Total: whowas 5756672 channel 1618438 conf 0",
    // whowas is the same as above, and so is the conf value
    total_channel_memory: usize,

    // "Local client Memory in use: 0(0)",
    local_client_count: usize,
    local_client_memory: usize,

    // "Remote client Memory in use: 0(0)"
    remote_client_count: usize,
    remote_client_memory: usize,

    // "TOTAL: 7377222"
    total: usize,
}

fn parse_number<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, usize, E> {
    u64.parse(input).map(|(s, v)| (s, v as usize))
}

fn values<'a, E: ParseError<&'a str>>(
    prefix: &'static str,
    input: &'a str,
) -> IResult<&'a str, (usize, usize), E> {
    let (r, _) = tag(prefix).parse(input)?;
    let (r, v) = parse_number.parse(r)?;
    let (r, v2) = delimited(tag("("), parse_number, tag(")")).parse(r)?;

    Ok((r, (v, v2)))
}

fn parse_line_4_values<'a, E: ParseError<&'a str>>(
    sep1: &'static str,
    sep2: &'static str,
    input: &'a str,
) -> IResult<&'a str, (usize, usize, usize, usize), E> {
    let (r, (v1, v2)) = values(sep1, input)?;
    let (r, (v3, v4)) = values(sep2, r)?;
    Ok((r, (v1, v2, v3, v4)))
}

fn parse_line_3_values<'a, E: ParseError<&'a str>>(
    sep1: &'static str,
    sep2: &'static str,
    sep3: &'static str,
    input: &'a str,
) -> IResult<&'a str, (usize, usize, usize), E> {
    let (r, _) = tag(sep1).parse(input)?;
    let (r, v1) = parse_number(r)?;
    let (r, _) = tag(sep2).parse(r)?;
    let (r, v2) = parse_number(r)?;
    let (r, _) = tag(sep3).parse(r)?;
    let (r, v3) = parse_number(r)?;
    Ok((r, (v1, v2, v3)))
}

// "Users 10175(1221000) Invites 0(0)"
#[inline]
fn parse_users_line<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, (usize, usize, usize, usize), E> {
    parse_line_4_values("Users ", " Invites ", input)
}
// "User channels 30651(735624) Aways 496(14282)",
#[inline]
fn parse_user_channels_line<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, (usize, usize, usize, usize), E> {
    parse_line_4_values("User channels ", " Aways ", input)
}

// "Attached confs 24(576)",
#[inline]
fn parse_attached_confs<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, (usize, usize), E> {
    values("Attached confs ", input)
}

// "Conflines 0(0)"
#[inline]
fn parse_conflines<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, (usize, usize), E> {
    values("Conflines ", input)
}

// "Classes 12(960)"
#[inline]
fn parse_classes<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, (usize, usize), E> {
    values("Classes ", input)
}

// "Channels 1988(816734)",
#[inline]
fn parse_channels<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, (usize, usize), E> {
    values("Channels ", input)
}

// "Bans 826(66080) Exceptions 31(2480) Invex 552(44160) Quiets 131(10480)",
#[inline]
fn parse_bans<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, (usize, usize, usize, usize, usize, usize, usize, usize), E> {
    let (
        r,
        (
            channel_ban_count,
            channel_ban_memory,
            channel_exceptions_count,
            channel_exceptions_memory,
        ),
    ) = parse_line_4_values("Bans ", " Exceptions ", input)?;

    let (
        r,
        (channel_invex_count, channel_invex_memory, channel_quiets_count, channel_quites_memory),
    ) = parse_line_4_values(" Invex ", " Quiets ", r)?;

    Ok((
        r,
        (
            channel_ban_count,
            channel_ban_memory,
            channel_exceptions_count,
            channel_exceptions_memory,
            channel_invex_count,
            channel_invex_memory,
            channel_quiets_count,
            channel_quites_memory,
        ),
    ))
}

// "Channel members 30651(735624) invite 0(0)",
#[inline]
fn parse_channel_members<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, (usize, usize, usize, usize), E> {
    parse_line_4_values("Channel members ", " invite ", input)
}

// "Whowas array 15000(5756672)",
#[inline]
fn parse_whowas<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, (usize, usize), E> {
    values("Whowas array ", input)
}

//"Hash: client 131072(3145728) chan 65536(1572864)"
#[inline]
fn parse_hash_line<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, (usize, usize, usize, usize), E> {
    parse_line_4_values("Hash: client ", " chan ", input)
}

#[inline]
fn parse_linebuf<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, (usize, usize), E> {
    values("linebuf ", input)
}

#[inline]
fn parse_scache_line<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, (usize, usize), E> {
    values("scache ", input)
}

#[inline]
fn parse_hostnames_line<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, (usize, usize), E> {
    values("hostname hash ", input)
}

#[inline]
fn parse_total_line<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, usize, E> {
    let (r, (_, total_channel_memory, _)) =
        parse_line_3_values("Total: whowas ", " channel ", " conf ", input)?;
    Ok((r, total_channel_memory))
}

#[inline]
fn parse_local_client_line<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, (usize, usize), E> {
    values("Local client Memory in use: ", input)
}

#[inline]
fn parse_remote_client_line<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, (usize, usize), E> {
    values("Remote client Memory in use: ", input)
}

#[inline]
fn parse_total_memory_line<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, usize, E> {
    let (r, _) = tag("TOTAL: ").parse(input)?;
    parse_number(r)
}

pub fn parse_stats_z(
    input: &str,
) -> Result<StatsZ, nom::Err<nom_language::error::VerboseError<&str>>> {
    let (r, (users, users_memory, users_invited, users_invited_memory)) = parse_users_line(input)?;
    let (r, _) = nom::character::complete::newline.parse(r)?;
    let (r, (user_channels, user_channels_memory, users_away, users_away_memory)) =
        parse_user_channels_line(r)?;
    let (r, _) = nom::character::complete::newline.parse(r)?;
    let (r, (local_client_conf_count, local_client_conf_memory)) = parse_attached_confs(r)?;
    let (r, _) = nom::character::complete::newline.parse(r)?;
    let (r, (conf_count, conf_memory)) = parse_conflines(r)?;
    let (r, _) = nom::character::complete::newline.parse(r)?;
    let (r, (classes_count, classes_memory)) = parse_classes(r)?;
    let (r, _) = nom::character::complete::newline.parse(r)?;
    let (r, (channels_count, channels_memory)) = parse_channels(r)?;
    let (r, _) = nom::character::complete::newline.parse(r)?;
    let (
        r,
        (
            channel_ban_count,
            channel_ban_memory,
            channel_exceptions_count,
            channel_exceptions_memory,
            channel_invex_count,
            channel_invex_memory,
            channel_quiets_count,
            channel_quiets_memory,
        ),
    ) = parse_bans(r)?;
    let (r, _) = nom::character::complete::newline.parse(r)?;
    let (
        r,
        (channel_members, channel_members_memory, channel_invites_count, channel_invites_memory),
    ) = parse_channel_members(r)?;
    let (r, _) = nom::character::complete::newline.parse(r)?;

    let (r, (whowas_count, whowas_memory)) = parse_whowas(r)?;
    let (r, _) = nom::character::complete::newline.parse(r)?;
    let (r, (hash_client_count, hash_client_memory, hash_channel_count, hash_channel_memory)) =
        parse_hash_line(r)?;
    let (r, _) = nom::character::complete::newline.parse(r)?;
    let (r, (linebuf_count, linebuf_memory)) = parse_linebuf(r)?;
    let (r, _) = nom::character::complete::newline.parse(r)?;
    let (r, (servers_cached_number, servers_cached_memory)) = parse_scache_line(r)?;
    let (r, _) = nom::character::complete::newline.parse(r)?;
    let (r, (hostname_count, hostname_memory)) = parse_hostnames_line(r)?;
    let (r, _) = nom::character::complete::newline.parse(r)?;
    let (r, total_channel_memory) = parse_total_line(r)?;
    let (r, _) = nom::character::complete::newline.parse(r)?;
    let (r, (local_client_count, local_client_memory)) = parse_local_client_line(r)?;
    let (r, _) = nom::character::complete::newline.parse(r)?;
    let (r, (remote_client_count, remote_client_memory)) = parse_remote_client_line(r)?;
    let (r, _) = nom::character::complete::newline.parse(r)?;
    let (r, total) = parse_total_memory_line(r)?;

    assert_eq!(r, "");

    Ok(StatsZ {
        users,
        users_memory,
        users_invited,
        users_invited_memory,
        user_channels,
        user_channels_memory,
        users_away,
        users_away_memory,
        local_client_conf_count,
        local_client_conf_memory,
        conf_count,
        conf_memory,
        classes_count,
        classes_memory,
        channels_count,
        channels_memory,
        channel_ban_count,
        channel_ban_memory,
        channel_exceptions_count,
        channel_exceptions_memory,
        channel_invex_count,
        channel_invex_memory,
        channel_quiets_count,
        channel_quiets_memory,
        channel_members,
        channel_members_memory,
        channel_invites_count,
        channel_invites_memory,
        whowas_count,
        whowas_memory,
        hash_client_count,
        hash_client_memory,
        hash_channel_count,
        hash_channel_memory,
        linebuf_count,
        linebuf_memory,
        servers_cached_number,
        servers_cached_memory,
        hostname_count,
        hostname_memory,
        total_channel_memory,
        local_client_count,
        local_client_memory,
        remote_client_count,
        remote_client_memory,
        total,
    })
}

#[cfg(test)]
mod tests {
    // /stats z lines: [
    // "Users 10175(1221000) Invites 0(0)"
    // "User channels 30651(735624) Aways 496(14282)",
    // "Attached confs 24(576)",
    // "Conflines 0(0)",
    // "Classes 12(960)",
    // "Channels 1988(816734)",
    // "Bans 826(66080) Exceptions 31(2480) Invex 552(44160) Quiets 131(10480)",
    // "Channel members 30651(735624) invite 0(0)",
    // "Whowas array 15000(5756672)",
    // "Hash: client 131072(3145728) chan 65536(1572864)",
    // "linebuf 0(0)",
    // "scache 8(1152)",
    // "hostname hash 131072(3145728)",
    // "Total: whowas 5756672 channel 1618438 conf 0",
    // "Local client Memory in use: 0(0)",
    // "Remote client Memory in use: 0(0)",
    // "TOTAL: 7377222"
    // ]

    use simple_prometheus::SimplePrometheus;

    #[test]
    fn test_parse_statsz() {
        let lines = vec![
            "Users 10175(1221000) Invites 0(0)",
            "User channels 30651(735624) Aways 496(14282)",
            "Attached confs 24(576)",
            "Conflines 0(0)",
            "Classes 12(960)",
            "Channels 1988(816734)",
            "Bans 826(66080) Exceptions 31(2480) Invex 552(44160) Quiets 131(10480)",
            "Channel members 30651(735624) invite 0(0)",
            "Whowas array 15000(5756672)",
            "Hash: client 131072(3145728) chan 65536(1572864)",
            "linebuf 0(0)",
            "scache 8(1152)",
            "hostname hash 131072(3145728)",
            "Total: whowas 5756672 channel 1618438 conf 0",
            "Local client Memory in use: 0(0)",
            "Remote client Memory in use: 0(0)",
            "TOTAL: 7377222",
        ];
        let line = lines.join("\n");
        let r = super::parse_stats_z(&line).unwrap();
        assert_eq!(r.total, 7377222);

        println!(
            "{}",
            r.to_prometheus_metrics(Some("foobar".into())).unwrap()
        );
    }

    #[test]
    fn test_parse_users_line() {
        let line = "Users 10175(1221000) Invites 0(0)";
        let (r, (v1, v2, v3, v4)) =
            super::parse_users_line::<nom_language::error::VerboseError<&str>>(line).unwrap();
        assert_eq!(r, "");
        assert_eq!(v1, 10175);
        assert_eq!(v2, 1221000);
        assert_eq!(v3, 0);
        assert_eq!(v4, 0);
    }

    #[test]
    fn test_parse_user_channels_line() {
        let line = "User channels 30651(735624) Aways 496(14282)";
        let (r, (v1, v2, v3, v4)) =
            super::parse_user_channels_line::<nom_language::error::VerboseError<&str>>(line)
                .unwrap();
        assert_eq!(r, "");
        assert_eq!(v1, 30651);
        assert_eq!(v2, 735624);
        assert_eq!(v3, 496);
        assert_eq!(v4, 14282);
    }

    #[test]
    fn test_parse_attached_confs_line() {
        let line = "Attached confs 24(576)";
        let (r, (v1, v2)) =
            super::parse_attached_confs::<nom_language::error::VerboseError<&str>>(line).unwrap();
        assert_eq!(r, "");
        assert_eq!(v1, 24);
        assert_eq!(v2, 576);
    }

    #[test]
    fn test_parse_bans() {
        let line = "Bans 826(66080) Exceptions 31(2480) Invex 552(44160) Quiets 131(10480)";
        let (r, (v1, v2, v3, v4, v5, v6, v7, v8)) =
            super::parse_bans::<nom_language::error::VerboseError<&str>>(line).unwrap();
        assert_eq!(r, "");
        assert_eq!(v1, 826);
        assert_eq!(v2, 66080);
    }

    #[test]
    fn test_parse_channel_members() {
        let line = "Channel members 30651(735624) invite 0(0)";
        let (r, (v1, v2, v3, v4)) =
            super::parse_channel_members::<nom_language::error::VerboseError<&str>>(line).unwrap();
        assert_eq!(r, "");
        assert_eq!(v1, 30651);
        assert_eq!(v2, 735624);
        assert_eq!(v3, 0);
        assert_eq!(v4, 0);
    }

    #[test]
    fn test_parse_whowas() {
        let line = "Whowas array 15000(5756672)";
        let (r, (v1, v2)) =
            super::parse_whowas::<nom_language::error::VerboseError<&str>>(line).unwrap();
        assert_eq!(r, "");
        assert_eq!(v1, 15000);
        assert_eq!(v2, 5756672);
    }

    #[test]
    fn test_parse_hash_line() {
        let line = "Hash: client 131072(3145728) chan 65536(1572864)";
        let (r, (v1, v2, v3, v4)) =
            super::parse_hash_line::<nom_language::error::VerboseError<&str>>(line).unwrap();
        assert_eq!(r, "");
        assert_eq!(v1, 131072);
        assert_eq!(v2, 3145728);
        assert_eq!(v3, 65536);
        assert_eq!(v4, 1572864);
    }
}
