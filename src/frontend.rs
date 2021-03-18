// Copyright 2020 - 2021 Alex Dukhno
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{PgFormat, PgType};

/// Frontend message that could be received during connection hand shake process
#[derive(Debug, PartialEq)]
pub enum HandShakeMessage {
    /// Client requested GSSENC Request
    GssencRequest,
    /// Client requested SSL connection
    SslRequest,
    /// Connection setup message
    Setup {
        /// client parameters
        params: Vec<(String, String)>,
    },
}

/// Frontend message that could be received during client server communication
#[derive(Debug, PartialEq)]
pub enum CommandMessage {
    /// Execute the specified SQL.
    ///
    /// This is issued as part of the simple query flow.
    Query {
        /// The SQL to execute.
        sql: String,
    },

    /// Parse the specified SQL into a prepared statement.
    ///
    /// This starts the extended query flow.
    Parse {
        /// The name of the prepared statement to create. An empty string
        /// specifies the unnamed prepared statement.
        statement_name: String,
        /// The SQL to parse.
        sql: String,
        /// The number of specified parameter data types can be less than the
        /// number of parameters specified in the query.
        param_types: Vec<Option<PgType>>,
    },

    /// Describe an existing prepared statement.
    ///
    /// This command is part of the extended query flow.
    DescribeStatement {
        /// The name of the prepared statement to describe.
        name: String,
    },

    /// Describe an existing portal.
    ///
    /// This command is part of the extended query flow.
    DescribePortal {
        /// The name of the portal to describe.
        name: String,
    },

    /// Bind an existing prepared statement to a portal.
    ///
    /// This command is part of the extended query flow.
    Bind {
        /// The destination portal. An empty string selects the unnamed
        /// portal. The portal can later be executed with the `Execute` command.
        portal_name: String,
        /// The source prepared statement. An empty string selects the unnamed
        /// prepared statement.
        statement_name: String,
        /// The formats used to encode the parameters.
        param_formats: Vec<PgFormat>,
        /// The value of each parameter.
        raw_params: Vec<Option<Vec<u8>>>,
        /// The desired formats for the columns in the result set.
        result_formats: Vec<PgFormat>,
    },

    /// Execute a bound portal.
    ///
    /// This command is part of the extended query flow.
    Execute {
        /// The name of the portal to execute.
        portal_name: String,
        /// The maximum number of rows to return before suspending.
        ///
        /// 0 or negative means infinite.
        max_rows: i32,
    },

    /// Flush any pending output.
    ///
    /// This command is part of the extended query flow.
    Flush,

    /// Finish an extended query.
    ///
    /// This command is part of the extended query flow.
    Sync,

    /// Close the named statement.
    ///
    /// This command is part of the extended query flow.
    CloseStatement {
        /// The name of the prepared statement to close.
        name: String,
    },

    /// Close the named portal.
    ///
    /// This command is part of the extended query flow.
    ClosePortal {
        /// The name of the portal to close.
        name: String,
    },

    /// Terminate a connection.
    Terminate,
}
