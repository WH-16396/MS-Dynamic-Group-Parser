## Notes

Link for official Docs: https://learn.microsoft.com/en-gb/entra/identity/users/groups-dynamic-membership

*ALL OPERATORS FROM DOCS:*

| Operator           | Syntax         |
| ------------------ | -------------- |
| Add                | -plus          |
| All                | -all           |
| And                | -and           |
| Any                | -any           |
| Contains           | -contains      |
| EndsWith           | -endsWith      |
| Equals             | -eq            |
| GreaterThanOrEqual | -ge            |
| In                 | -in            |
| LessThanOrEqual    | -le            |
| Match              | -match         |
| Not                | -not           |
| NotContains        | -notContains   |
| NotEndsWith        | -notEndsWith   |
| NotEquals          | -ne            |
| NotIn              | -notIn         |
| NotMatch           | -notMatch      |
| NotStartsWith      | -notStartsWith |
| Or                 | -or            |
| StartsWith         | -startsWith    |
| Subtract           | -minus         |

- Operators are not case sensitive and do not need the '-' at the start
- Not all operators can go in the same positions
- '-not' can go before or after any condition as well as before the operator in the middle of a condition, they will also cancel each other out

## UNSUPPORTED FEATURES

- Date/Time property
- '-not' operator
- (_) syntax

## ARCHITECTURE OF THE API

The API will break down any syntax down into individual rules for each set of access to accurately compare the rules and then reconstruct this later back into a tree format with branches.

Example rule:

```(user.accountEnabled -eq True and user.userType -eq "Member") and (user.jobTitle -eq "job 1" or user.jobTitle -eq "job 2")```

When going through the API this would become two distinct rules: 

1. ```user.accountEnabled -eq True and user.userType -eq "Member" and user.jobTitle -eq "job 1"```
2. ```user.accountEnabled -eq True and user.userType -eq "Member" and user.jobTitle -eq "job 2"```

If it's chosen to output back into compiled syntax or a JSON tree structure this would then be merged back into a tree structure, combining any shared parents as well as into arrays where possible. This makes the rules agnostic to the structure of the input and works around some issues that may have been introduced as the rules have been modified over time.

Output syntax:

```user.accountEnabled -eq True and user.userType -eq "Member" and user.jobTitle -in ["job 1", "job 2"]```

## TEST CASE

(user.accountEnabled -eq True) and (user.userType -eq "Member") and ((user.department -eq "d1" and (user.jobTitle -eq "d1j1" or user.jobTitle -eq "d1j2" or user.jobTitle -eq "d1j3" or user.jobTitle -eq "d1j3")) or (user.department -eq "d2" and (user.jobTitle -eq "d2j1" or user.jobTitle -eq "d2j2" or user.jobTitle -eq "d2j3")) or (user.department -eq "d3" and (user.jobTitle -eq "d3j1" or user.jobTitle -eq "d3j2" or user.jobTitle -eq "d3j3")) or (user.department -eq "d4" and (user.jobTitle -eq "d4j1")) or (user.department -eq "d5" and (user.jobTitle -eq "d5j1")) or (user.department -eq "d6" and (user.jobTitle -eq "d6j1" or user.jobTitle -eq "d6j2" or user.jobTitle -eq "d6j3" or user.jobTitle -eq "d6j4")) or (user.department -eq "d7" and (user.jobTitle -eq "d7j1")))

```"(user.accountEnabled -eq True) and (user.userType -eq \"Member\") and ((user.department -eq \"d1\" and (user.jobTitle -eq \"d1j1\" or user.jobTitle -eq \"d1j2\" or user.jobTitle -eq \"d1j3\" or user.jobTitle -eq \"d1j3\")) or (user.department -eq \"d2\" and (user.jobTitle -eq \"d2j1\" or user.jobTitle -eq \"d2j2\" or user.jobTitle -eq \"d2j3\")) or (user.department -eq \"d3\" and (user.jobTitle -eq \"d3j1\" or user.jobTitle -eq \"d3j2\" or user.jobTitle -eq \"d3j3\")) or (user.department -eq \"d4\" and (user.jobTitle -eq \"d4j1\")) or (user.department -eq \"d5\" and (user.jobTitle -eq \"d5j1\")) or (user.department -eq \"d6\" and (user.jobTitle -eq \"d6j1\" or user.jobTitle -eq \"d6j2\" or user.jobTitle -eq \"d6j3\" or user.jobTitle -eq \"d6j4\")) or (user.department -eq \"d7\" and (user.jobTitle -eq \"d7j1\")))"```