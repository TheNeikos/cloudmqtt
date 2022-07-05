//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use nom::{multi::many1_count, Parser};

use super::{
    strings::{mstring, MString},
    MSResult,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MUnsubscriptionRequests<'message> {
    count: usize,
    data: &'message [u8],
}

impl<'message> IntoIterator for MUnsubscriptionRequests<'message> {
    type Item = MUnsubscriptionRequest<'message>;

    type IntoIter = MUnsubscriptionIter<'message>;

    fn into_iter(self) -> Self::IntoIter {
        MUnsubscriptionIter {
            count: self.count,
            data: self.data,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MUnsubscriptionIter<'message> {
    count: usize,
    data: &'message [u8],
}

impl<'message> Iterator for MUnsubscriptionIter<'message> {
    type Item = MUnsubscriptionRequest<'message>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }

        self.count -= 1;
        let (rest, request) = munsubscriptionrequest(self.data)
            .expect("Could not parse already validated sub request");
        self.data = rest;

        Some(request)
    }
}

pub fn munsubscriptionrequests(input: &[u8]) -> MSResult<'_, MUnsubscriptionRequests> {
    let data = input;
    let (input, count) = many1_count(munsubscriptionrequest)(input)?;

    Ok((input, MUnsubscriptionRequests { count, data }))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MUnsubscriptionRequest<'message> {
    pub topic: MString<'message>,
}

fn munsubscriptionrequest(input: &[u8]) -> MSResult<'_, MUnsubscriptionRequest> {
    mstring
        .map(|topic| MUnsubscriptionRequest { topic })
        .parse(input)
}
