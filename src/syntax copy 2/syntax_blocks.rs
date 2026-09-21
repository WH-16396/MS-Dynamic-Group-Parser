pub enum DataType {
    String(String),
    Boolean(bool),
    Number(i32),
    Null,
    // DateTime(DateTime),
}
// pub enum DateTime {
//     DateTimeOffset {
//         year: u8,
//         month: u8,
//         day: u8,
//         hour: u8,
//         minute: u8,
//         second: u8,
//     },
//     DateOffset {
//         year: u8,
//         month: u8,
//         day: u8,
//     },
// }
pub enum Object {
    User,
    Device,
    Other(&'static str),
}
pub enum PropertyOption {
    City,
    Country,
    CompanyName,
    Department,
    DisplayName,
    EmployeeId,
    FacsimileTelephoneNumber,
    GivenName,
    JobTitle,
    Mail,
    MailNickName,
    Mobile,
    ObjectId,
    OnPremisesDistinguishedName,
    OnPremisesSecurityIdentifier,
    PasswordPolicies,
    PhysicalDeliveryOfficeName,
    PostalCode,
    PreferredLanguage,
    SipProxyAddress,
    State,
    StreetAddress,
    Surname,
    TelephoneNumber,
    UsageLocation,
    UserPrincipalName,
    UserType,
    Other(&'static str, Vec<DataType>),
}

pub struct Property {
    pub object: Object,
    pub property: &'static str,
}