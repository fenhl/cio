use std::collections::BTreeMap;
use std::collections::HashMap;

use gsuite_api::{
    generate_password, GSuite, Group as GSuiteGroup, User as GSuiteUser, UserAddress, UserCustomProperties, UserEmail, UserGender, UserInstantMessenger, UserLocation, UserName, UserPhone, UserSSHKey,
};
use serde_json::Value;
use tracing::{event, instrument, Level};

use crate::configs::User;
use crate::utils::GSUITE_DOMAIN;

/// Update a GSuite user.
#[instrument]
#[inline]
pub async fn update_gsuite_user(gu: &GSuiteUser, user: &User, change_password: bool) -> GSuiteUser {
    let mut gsuite_user = gu.clone();

    gsuite_user.name = UserName {
        full_name: format!("{} {}", user.first_name, user.last_name),
        given_name: user.first_name.to_string(),
        family_name: user.last_name.to_string(),
    };

    if !user.recovery_email.is_empty() {
        // Set the recovery email for the user.
        gsuite_user.recovery_email = user.recovery_email.to_string();

        // Check if we have a home email set for the user and update it.
        let mut has_home_email = false;
        for (index, email) in gsuite_user.emails.iter().enumerate() {
            if email.typev == "home" {
                // Update the set home email.
                gsuite_user.emails[index].address = user.recovery_email.to_string();
                // Break the loop early.
                has_home_email = true;
                break;
            }
        }

        if !has_home_email {
            // Set the home email for the user.
            gsuite_user.emails.push(UserEmail {
                custom_type: "".to_string(),
                typev: "home".to_string(),
                address: user.recovery_email.to_string(),
                primary: false,
            });
        }
    }

    if !user.recovery_phone.is_empty() {
        // Set the recovery phone for the user.
        gsuite_user.recovery_phone = user.recovery_phone.to_string();

        // Set the home phone for the user.
        gsuite_user.phones = vec![UserPhone {
            custom_type: "".to_string(),
            typev: "home".to_string(),
            value: user.recovery_phone.to_string(),
            primary: true,
        }];
    }

    gsuite_user.primary_email = format!("{}@{}", user.username, GSUITE_DOMAIN);

    if change_password {
        // Since we are creating a new user, we want to change their password
        // at the next login.
        gsuite_user.change_password_at_next_login = true;
        // Generate a password for the user.
        let password = generate_password();
        gsuite_user.password = password;
    }

    // Set the user's address if we have one.
    if !user.home_address_street_1.is_empty() {
        // TODO: this code is duplicated in configs.rs find a way to make it DRY.
        let mut street_address = user.home_address_street_1.to_string();
        if !user.home_address_street_2.is_empty() {
            street_address = format!("{}\n{}", user.home_address_street_1, user.home_address_street_2,);
        }
        gsuite_user.addresses = vec![UserAddress {
            country: user.home_address_country.to_string(),
            // TODO: fix this when we have an employee from another country.
            country_code: "US".to_string(),
            custom_type: "".to_string(),
            extended_address: "".to_string(),
            formatted: user.home_address_formatted.to_string(),
            locality: user.home_address_city.to_string(),
            po_box: "".to_string(),
            postal_code: user.home_address_zipcode.to_string(),
            primary: true,
            region: user.home_address_state.to_string(),
            // Indicates if the user-supplied address was formatted. Formatted addresses are
            // not currently supported.
            // FROM: https://developers.google.com/admin-sdk/directory/v1/reference/users#resource
            // TODO: figure out when this is supported and what it means
            source_is_structured: false,
            street_address,
            typev: "home".to_string(),
        }];
    }

    // Include the user in the global address list
    gsuite_user.include_in_global_address_list = true;

    if !user.gender.is_empty() {
        gsuite_user.gender = Some(UserGender {
            address_me_as: "".to_string(),
            custom_gender: "".to_string(),
            typev: user.gender.to_string(),
        });
    }

    if !user.building.is_empty() {
        gsuite_user.locations = vec![UserLocation {
            area: "".to_string(),
            building_id: user.building.to_string(),
            custom_type: "".to_string(),
            desk_code: "".to_string(),
            floor_name: "1".to_string(),
            floor_section: "".to_string(),
            typev: "desk".to_string(),
        }];
    }

    // Set their GitHub SSH Keys to their Google SSH Keys.
    for k in &user.public_ssh_keys {
        gsuite_user.ssh_public_keys.push(UserSSHKey {
            key: k.to_string(),
            expiration_time_usec: None,
            // fingerprint is a read-only property so make sure it is empty
            fingerprint: "".to_string(),
        });
    }

    // Set the IM field for matrix.
    // TODO: once we migrate to slack update or add to this.
    if !user.chat.is_empty() {
        gsuite_user.ims = vec![UserInstantMessenger {
            custom_protocol: "matrix".to_string(),
            custom_type: "".to_string(),
            im: user.chat.to_string(),
            primary: true,
            protocol: "custom_protocol".to_string(),
            typev: "work".to_string(),
        }];
    }

    // Set the custom schemas.
    gsuite_user.custom_schemas = HashMap::new();
    let mut contact: HashMap<String, Value> = HashMap::new();
    contact.insert("Start_Date".to_string(), json!(user.start_date));

    // Set the GitHub username.
    if !user.github.is_empty() {
        contact.insert("GitHub_Username".to_string(), json!(user.github.to_string()));
    }
    gsuite_user.custom_schemas.insert("Contact".to_string(), UserCustomProperties(Some(contact)));

    // Get the AWS Role information.
    if !user.aws_role.is_empty() {
        let mut aws_role: HashMap<String, Value> = HashMap::new();
        let mut aws_type: HashMap<String, String> = HashMap::new();
        aws_type.insert("type".to_string(), "work".to_string());
        aws_type.insert("value".to_string(), user.aws_role.to_string());
        aws_role.insert("Role".to_string(), json!(vec![aws_type]));
        gsuite_user.custom_schemas.insert("Amazon_Web_Services".to_string(), UserCustomProperties(Some(aws_role)));
    }

    gsuite_user
}

/// Update a user's aliases in GSuite to match our database.
#[instrument(skip(gsuite))]
#[inline]
pub async fn update_user_aliases(gsuite: &GSuite, u: &GSuiteUser, aliases: Vec<String>) {
    if aliases.is_empty() {
        // Return early.
        return;
    }

    let mut formatted_aliases: Vec<String> = Default::default();
    for a in aliases {
        formatted_aliases.push(format!("{}@{}", a, GSUITE_DOMAIN));
    }

    // Update the user's aliases.
    gsuite.update_user_aliases(&u.primary_email, formatted_aliases).await;
    event!(Level::INFO, "updated gsuite user aliases: {}", u.primary_email);
}

/// Update a user's groups in GSuite to match our database.
#[instrument(skip(gsuite))]
#[inline]
pub async fn update_user_google_groups(gsuite: &GSuite, user: &User, google_groups: BTreeMap<String, GSuiteGroup>) {
    // Iterate over the groups and add the user as a member to it.
    for g in &user.groups {
        // Make sure the group exists.
        let group: &GSuiteGroup;
        match google_groups.get(g) {
            Some(val) => group = val,
            // Continue through the loop and we will add the user later.
            None => panic!("google group {} does not exist so cannot add user {}", g, user.email()),
        }

        let mut role = "MEMBER";
        if user.is_group_admin {
            role = "OWNER";
        }

        // Check if the user is already a member of the group.
        let is_member = gsuite.group_has_member(&group.id, &user.email()).await.unwrap();
        if is_member {
            // They are a member so we can just update their member status.
            gsuite.group_update_member(&group.id, &user.email(), &role).await.unwrap();

            // Continue through the other groups.
            continue;
        }

        // Add the user to the group.
        gsuite.group_insert_member(&group.id, &user.email(), &role).await.unwrap();

        event!(Level::INFO, "added {} to gsuite group {} as {}", user.email(), group.name, role);
    }

    // Iterate over all the groups and if the user is a member and should not
    // be, remove them from the group.
    for (slug, group) in &google_groups {
        if user.groups.contains(&slug) {
            continue;
        }

        // Now we have a google group. The user should not be a member of it,
        // but we need to make sure they are not a member.
        let is_member = gsuite.group_has_member(&group.id, &user.email()).await.unwrap();

        if !is_member {
            // They are not a member so we can continue early.
            continue;
        }

        // They are a member of the group.
        // We need to remove them.
        gsuite.group_remove_member(&group.id, &user.email()).await.unwrap();

        event!(Level::INFO, "removed {} from gsuite group {}", user.email(), group.name);
    }
}