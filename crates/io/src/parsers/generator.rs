// Parsers for parsing out and instanciating
// combinations ofgenerators and stop condintions
// For parsing nom will be used. Some relevant articles:
//  - https://blog.adamchalmers.com/nom-chars/

// #[derive(Debug, PartialEq)]
// pub struct StopCondition {
//     pub name: str,
// }

// #[derive(Debug, PartialEq)]
// pub struct Generator {
//     pub name: str,
//     pub condition: StopCondition,
// }
use parse_hyperlinks::take_until_unbalanced;

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alpha1, alphanumeric1},
    combinator::recognize,
    multi::many0_count,
    sequence::pair,
    IResult,
};

pub fn identifier(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        alt((alpha1, tag("_"))),
        many0_count(alt((alphanumeric1, tag("_")))),
    ))(input)
}

// fn decimal(input: &str) -> IResult<&str, &str> {
//     recognize(many1(terminated(one_of("0123456789"), many0(char('_')))))(input)
// }

// fn left_parantheses(input: &str) -> IResult<&str, &str> {
//     tag("(")(input)
// }

// fn right_parantheses(input: &str) -> IResult<&str, &str> {
//     tag(")")(input)
// }

fn generator(input: &str) -> IResult<&str, &str> {
    let (rest, generator) = identifier(input)?;

    // get everything inside the random parantheses
    let (_rest, inner) =
        nom::sequence::delimited(tag("("), take_until_unbalanced('(', ')'), tag(")"))(rest)?;
    IResult::Ok((generator, inner))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::error::ErrorKind;
    use pretty_assertions::assert_eq;

    #[test]
    fn generator_test() {
        assert_eq!(
            generator("quick_random(edge_coverage(100))"),
            Ok(("quick_random", "edge_coverage(100)"))
        );
        assert_eq!(
            generator("random(edge_coverage(100))"),
            Ok(("random", "edge_coverage(100)"))
        );
        assert_eq!(
            generator("random(edge_coverage(100) OR reached_vertex(v1))"),
            Ok(("random", "edge_coverage(100) OR reached_vertex(v1)"))
        );
        assert_eq!(
            generator("random(edge_coverage(100) OR (reached_vertex(v1) AND time(800)))"),
            Ok((
                "random",
                "edge_coverage(100) OR (reached_vertex(v1) AND time(800))"
            ))
        );
    }

    #[test]
    fn random_generator_test() {
        assert_eq!(
            take_until_unbalanced('(', ')')("edge_coverage(100))"),
            Ok((")", "edge_coverage(100)"))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("edge_coverage(100) or vertex_reached(v1))"),
            Ok((")", "edge_coverage(100) or vertex_reached(v1)"))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("100) or vertex_reached(v1))"),
            Ok((") or vertex_reached(v1))", "100"))
        );
        assert_eq!(take_until_unbalanced('(', ')')("v1))"), Ok(("))", "v1")));
    }

    #[test]
    fn hyperlink_test() {
        assert_eq!(take_until_unbalanced('(', ')')("abc"), Ok(("", "abc")));
        assert_eq!(
            take_until_unbalanced('(', ')')("url)abc"),
            Ok((")abc", "url"))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("u()rl)abc"),
            Ok((")abc", "u()rl"))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("u(())rl)abc"),
            Ok((")abc", "u(())rl"))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("u(())r()l)abc"),
            Ok((")abc", "u(())r()l"))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("u(())r()labc"),
            Ok(("", "u(())r()labc"))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')(r#"u\((\))r()labc"#),
            Ok(("", r#"u\((\))r()labc"#))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("u(())r(labc"),
            Err(nom::Err::Error(nom::error::Error::new(
                "u(())r(labc",
                ErrorKind::TakeUntil
            )))
        );
        assert_eq!(
            take_until_unbalanced('€', 'ü')("€uü€€üürlüabc"),
            Ok(("üabc", "€uü€€üürl"))
        );
        //     let res = identifier("random(edge_coverage(100))");
        //     assert_eq!(res, Ok(("(edge_coverage(100))", "random")));

        //     let res = left_parantheses(res.unwrap().0);
        //     assert_eq!(res, Ok(("edge_coverage(100))", "(")));

        //     let res = identifier(res.unwrap().0);
        //     assert_eq!(res, Ok(("(100))", "edge_coverage")));

        //     let res = left_parantheses(res.unwrap().0);
        //     assert_eq!(res, Ok(("100))", "(")));

        //     let res = decimal(res.unwrap().0);
        //     assert_eq!(res, Ok(("))", "100")));

        //     let res = right_parantheses(res.unwrap().0);
        //     assert_eq!(res, Ok((")", ")")));

        //     let res = right_parantheses(res.unwrap().0);
        //     assert_eq!(res, Ok(("", ")")));
    }
}
