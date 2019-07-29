/*
 * Copyright 2018-2019 Andreas Nordal
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ::situation::Situation;
use ::situation::Transition;
use ::situation::WhatNow;
use ::situation::ParseResult;
use ::situation::flush;
use ::situation::COLOR_NORMAL;
use ::situation::COLOR_KWD;

use ::microparsers::predlen;
use ::microparsers::is_whitespace;
use ::microparsers::is_word;

use ::commonargcmd::keyword_or_command;
use ::commonargcmd::common_no_cmd_quoting_unneeded;

pub struct SitIn {}

impl Situation for SitIn {
	fn whatnow(&mut self, horizon: &[u8], is_horizon_lengthenable: bool) -> ParseResult {
		for (i, _) in horizon.iter().enumerate() {
			let len = predlen(&is_word, &horizon[i..]);
			if len == 0 {
				continue;
			}
			if i + len == horizon.len() && (i > 0 || is_horizon_lengthenable) {
				return Ok(flush(i));
			}
			let word = &horizon[i..i+len];
			if word == b"in" {
				return Ok(WhatNow{
					tri: Transition::Replace(Box::new(SitCase{})),
					pre: i + len, len: 0, alt: None
				});
			}
			if let Some(res) = common_no_cmd_quoting_unneeded(
				0x100, horizon, i, is_horizon_lengthenable
			) {
				return res;
			}
			return Ok(flush(i + len));
		}
		Ok(flush(horizon.len()))
	}
	fn get_color(&self) -> u32 {
		COLOR_KWD
	}
}

struct SitCase {}

impl Situation for SitCase {
	fn whatnow(&mut self, horizon: &[u8], is_horizon_lengthenable: bool) -> ParseResult {
		for (i, &a) in horizon.iter().enumerate() {
			let len = predlen(&is_word, &horizon[i..]);
			if len == 0 {
				if a == b')' {
					return Ok(WhatNow{
						tri: Transition::Push(Box::new(SitCaseArm{})),
						pre: i, len: 1, alt: None
					});
				}
				continue;
			}
			if i + len == horizon.len() && (i > 0 || is_horizon_lengthenable) {
				return Ok(flush(i));
			}
			let word = &horizon[i..i+len];
			if word == b"esac" {
				return Ok(WhatNow{
					tri: Transition::Pop, pre: i, len: 0, alt: None
				});
			}
			if let Some(res) = common_no_cmd_quoting_unneeded(
				0x100, horizon, i, is_horizon_lengthenable
			) {
				return res;
			}
			return Ok(flush(i + len));
		}
		Ok(flush(horizon.len()))
	}
	fn get_color(&self) -> u32 {
		COLOR_NORMAL
	}
}

struct SitCaseArm {}

impl Situation for SitCaseArm {
	fn whatnow(&mut self, horizon: &[u8], is_horizon_lengthenable: bool) -> ParseResult {
		for (i, &a) in horizon.iter().enumerate() {
			if a == b';' {
				if i + 1 < horizon.len() {
					if horizon[i + 1] == b';' {
						return Ok(WhatNow{
							tri: Transition::Pop, pre: i, len: 0, alt: None
						});
					}
				} else if i > 0 || is_horizon_lengthenable {
					return Ok(flush(i));
				}
			}
			if is_whitespace(a) || a == b';' || a == b'|' || a == b'&' || a == b'<' || a == b'>' {
				continue;
			}
			// Premature esac: Survive and rewrite.
			let len = predlen(&is_word, &horizon[i..]);
			if i + len != horizon.len() || (i == 0 && !is_horizon_lengthenable) {
				let word = &horizon[i..i+len];
				if word == b"esac" {
					return Ok(WhatNow{
						tri: Transition::Pop, pre: i, len: 0, alt: Some(b";; ")
					});
				}
			}
			return Ok(keyword_or_command(0x100, &horizon, i, is_horizon_lengthenable));
		}
		Ok(flush(horizon.len()))
	}
	fn get_color(&self) -> u32 {
		COLOR_NORMAL
	}
}

#[cfg(test)]
use ::testhelpers::*;
#[cfg(test)]
use ::sitcmd::SitCmd;

#[test]
fn test_sit_in() {
	sit_expect!(SitIn{}, b"", &Ok(flush(0)));
	sit_expect!(SitIn{}, b" ", &Ok(flush(1)));
	sit_expect!(SitIn{}, b"i", &Ok(flush(0)), &Ok(flush(1)));
	let found_the_in_word = Ok(WhatNow{
		tri: Transition::Replace(Box::new(SitCase{})),
		pre: 2, len: 0, alt: None
	});
	sit_expect!(SitIn{}, b"in ", &found_the_in_word);
	sit_expect!(SitIn{}, b"in", &Ok(flush(0)), &found_the_in_word);
	sit_expect!(SitIn{}, b"inn", &Ok(flush(0)), &Ok(flush(3)));
	sit_expect!(SitIn{}, b" in", &Ok(flush(1)));
	sit_expect!(SitIn{}, b"fin", &Ok(flush(0)), &Ok(flush(3)));
	sit_expect!(SitIn{}, b"fin ", &Ok(flush(3)));
}

#[test]
fn test_sit_case() {
	sit_expect!(SitCase{}, b"", &Ok(flush(0)));
	sit_expect!(SitCase{}, b" ", &Ok(flush(1)));
	sit_expect!(SitCase{}, b"esa", &Ok(flush(0)), &Ok(flush(3)));
	let found_the_esac_word = Ok(WhatNow{
		tri: Transition::Pop,
		pre: 0, len: 0, alt: None
	});
	sit_expect!(SitCase{}, b"esac ", &found_the_esac_word);
	sit_expect!(SitCase{}, b"esac", &Ok(flush(0)), &found_the_esac_word);
	sit_expect!(SitCase{}, b"esacs", &Ok(flush(0)), &Ok(flush(5)));
	sit_expect!(SitCase{}, b" esac", &Ok(flush(1)));
	sit_expect!(SitCase{}, b"besac", &Ok(flush(0)), &Ok(flush(5)));
	sit_expect!(SitCase{}, b"besac ", &Ok(flush(5)));
}

#[test]
fn test_sit_casearm() {
	sit_expect!(SitCaseArm{}, b"", &Ok(flush(0)));
	sit_expect!(SitCaseArm{}, b" ", &Ok(flush(1)));
	let found_command = Ok(WhatNow{
		tri: Transition::Push(Box::new(SitCmd{end_trigger: 0x100})),
		pre: 0, len: 0, alt: None
	});
	sit_expect!(SitCaseArm{}, b"esa", &Ok(flush(0)), &found_command);
	let found_the_esac_word = Ok(WhatNow{
		tri: Transition::Pop,
		pre: 0, len: 0, alt: Some(b";; ")
	});
	sit_expect!(SitCaseArm{}, b"esac ", &found_the_esac_word);
	sit_expect!(SitCaseArm{}, b"esac", &Ok(flush(0)), &found_the_esac_word);
	sit_expect!(SitCaseArm{}, b"esacs", &Ok(flush(0)), &found_command);
	sit_expect!(SitCaseArm{}, b" esac", &Ok(flush(1)));
	sit_expect!(SitCaseArm{}, b"besac", &Ok(flush(0)), &found_command);
	sit_expect!(SitCaseArm{}, b"besac ", &found_command);
}
