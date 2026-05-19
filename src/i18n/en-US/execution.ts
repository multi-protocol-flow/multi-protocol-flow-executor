const en_US_execution = {
  node_types: {
    entry: 'Entry',
    http: 'HTTP',
    conditional: 'Conditional',
    script: 'Script',
    assertion: 'Assertion',
    end: 'End',
    ws_connect: 'WS-Connect',
    ws_send_collect: 'WS-Send/Collect',
    ws_close: 'WS-Close',
    db_connect: 'DB-Connect',
    db_query: 'DB-Query',
    db_begin: 'DB-Begin',
    db_commit: 'DB-Commit',
    db_rollback: 'DB-Rollback',
    db_close: 'DB-Close',
  },
  assertion: {
    operators: {
      equal: 'Equal',
      not_equal: 'Not Equal',
      greater: 'Greater Than',
      greater_or_equal: 'Greater or Equal',
      less: 'Less Than',
      less_or_equal: 'Less or Equal',
      contains: 'Contains',
      not_contains: 'Not Contains',
      starts_with: 'Starts With',
      ends_with: 'Ends With',
      exists: 'Exists',
      not_exists: 'Not Exists',
      is_empty: 'Is Empty',
      is_not_empty: 'Is Not Empty',
      matches_regex: 'Matches Regex',
    },
    modes: {
      all: 'All Passed (AND)',
      any: 'Any Passed (OR)',
    },
    failure_modes: {
      route_false: 'Route to False Branch',
      throw: 'Throw Exception and Terminate',
    },
    presets: {
      http_status: 'HTTP Status Code Check',
      http_success: 'HTTP Success Response',
      response_body_exists: 'Response Body Exists',
      json_field_exists: 'JSON Field Exists',
      empty_check: 'Empty Check',
    },
    descriptions: {
      status_200: 'Status code is 200',
      status_ge_200: 'Status code >= 200',
      status_lt_300: 'Status code < 300',
      body_exists: 'Response body exists',
      data_field_exists: 'Field "data" exists',
      body_not_empty: 'Response body is not empty',
    },
  },
  status: {
    success: 'Success',
    failed: 'Failed'
  }
};

export default en_US_execution;