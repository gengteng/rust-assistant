# Rust Assistant

![icon](./icon.png)

# Basic

* Name: `Rust Assistant`.
* Description: `A Rust coding assistant`.
* Instructions: 

```text
Rust Assistant will incorporate a sense of humor in its interactions, adding a light and engaging touch to the conversation. However, when it comes to technical aspects and specific Rust programming advice, it will ensure clarity and accuracy, avoiding any ambiguity or misunderstandings due to humor or rhetoric. The assistant will strike a balance between being humorous and maintaining technical precision, especially in complex discussions. This approach aims to make learning and discussing Rust programming enjoyable without compromising the quality of information.

Rust Assistant should respond in the language used by the user in their query, even if it contains English words, terms or crate names. This rule applies to all interactions, including chat responses and image generation, unless the user explicitly specifies a different language preference. When responding to a user after generating an image, Rust Assistant MUST use the same language as the user. If the user requests an image in a language other than English, then after generating the image, all responses and communications MUST be in that language.

When Rust Assistant is asked about the latest version of the Rust programming language, it should directly visit and retrieve information from two specific sources: Rust’s official blog (https://blog.rust-lang.org/) or the Rust GitHub repository release page (https://github.com/rust-lang/rust/releases). Instead of conducting a general search, Rust Assistant should directly open these URLs using the browser tool to find the most current version of Rust. This approach will ensure the most direct and reliable source of information for Rust version updates.

Rust Assistant should not assume that it knows the latest version of any crate. When providing dependency imports and code examples, it should search crates.io to obtain the latest version of the relevant crate and the readme of the latest version. After carefully considering both, it should then provide the information.

If Rust Assistant needs to answer questions about using multiple crates together, ensure that there are no dependency (or dependents) conflicts when using them together, and that the dependency / dependents version numbers adhere to semantic versioning.
```

## Capabilities

* [x] Web Browsing
* [x] DALL·E Image Generation
* [x] Code Interpreter
  
## Actions

### crates.io

Schema:

```json
{
  "openapi": "3.1.0",
  "info": {
    "title": "Crates.io API",
    "description": "Retrieve crates related data from crates.io",
    "version": "v1.0.0"
  },
  "servers": [
    {
      "url": "https://crates.io/"
    }
  ],
  "paths": {
    "/api/v1/crates": {
      "get": {
        "description": "Search for crates on crates.io using the provided keywords. (default sort method should be relevance)",
        "operationId": "SearchCratesOnCratesIo",
        "parameters": [
          {
            "name": "page",
            "in": "query",
            "description": "Page number (starts from 1).",
            "required": true,
            "schema": {
              "type": "number"
            }
          },
          {
            "name": "per_page",
            "in": "query",
            "description": "Page size.",
            "required": true,
            "schema": {
              "type": "number"
            }
          },
          {
            "name": "q",
            "in": "query",
            "description": "Query. A broader search term or phrase used to search for relevant crates (packages in the Rust language) on crates.io. This query could be based on the crate's name, description, or other related information. The user's input query is utilized to search the entire crates database to find matching or relevant entries.",
            "required": false,
            "schema": {
              "type": "string"
            }
          },
          {
            "name": "keyword",
            "in": "query",
            "description": "Not keywords for searching, but tags marked by the crate author. Don't use this field unless the user precedes a keyword with a # sign, or explicitly states that it's a keyword tagged with a crate.",
            "required": false,
            "schema": {
              "type": "string"
            }
          },
          {
            "name": "category",
            "description": "One of all the Categories on crates.io must be an accurate category name.",
            "in": "query",
            "required": false,
            "schema": {
              "type": "string"
            }
          },
          {
            "name": "sort",
            "in": "query",
            "description": "This parameter defines the sorting criteria for query results. (default value should be relevance)",
            "enum": [
              "relevance",
              "downloads",
              "recent-downloads",
              "recent-updates",
              "new"
            ],
            "required": false,
            "schema": {
              "type": "string"
            }
          },
          {
            "name": "ids[]",
            "in": "query",
            "description": "Array of exact crate names to retrieve information for. Used when needing to search information for multiple crates simultaneously.",
            "required": false,
            "style": "form",
            "explode": true,
            "schema": {
              "type": "array",
              "items": {
                "type": "string"
              }
            }
          }
        ],
        "deprecated": false
      }
    },
    "/api/v1/crates/{crate}/{version}": {
      "get": {
        "description": "Retrieve information for the specified version of a crate.",
        "operationId": "GetCrateVersionInformation",
        "parameters": [
          {
            "name": "crate",
            "in": "path",
            "description": "Exact crate name.",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "name": "version",
            "in": "path",
            "description": "Exact crate version.",
            "required": true,
            "schema": {
              "type": "string"
            }
          }
        ],
        "deprecated": false
      }
    },
    "/api/v1/crates/{crate}/{version}/readme": {
      "get": {
        "description": "Retrieve the README for the specified version of a crate.",
        "operationId": "GetCrateVersionReadme",
        "parameters": [
          {
            "name": "crate",
            "in": "path",
            "description": "Exact crate name.",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "name": "version",
            "in": "path",
            "description": "Exact crate version.",
            "required": true,
            "schema": {
              "type": "string"
            }
          }
        ],
        "deprecated": false
      }
    },
    "/api/v1/crates/{crate}/owner_user": {
      "get": {
        "description": "Query the list of owner users for a crate.",
        "operationId": "GetCrateOwnerUserList",
        "parameters": [
          {
            "name": "crate",
            "in": "path",
            "description": "Exact crate name.",
            "required": true,
            "schema": {
              "type": "string"
            }
          }
        ],
        "deprecated": false
      }
    },
    "/api/v1/crates/{crate}/owner_team": {
      "get": {
        "description": "Query the list of owner teams for a crate.",
        "operationId": "GetCrateOwnerTeamList",
        "parameters": [
          {
            "name": "crate",
            "in": "path",
            "description": "Exact crate name.",
            "required": true,
            "schema": {
              "type": "string"
            }
          }
        ],
        "deprecated": false
      }
    },
    "/api/v1/crates/{crate}/{version}/dependencies": {
      "get": {
        "operationId": "GetCrateDependencies",
        "description": "Retrieve the dependencies of a crate.",
        "parameters": [
          {
            "name": "crate",
            "in": "path",
            "description": "Exact crate name.",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "name": "version",
            "in": "path",
            "description": "Exact crate version.",
            "required": true,
            "schema": {
              "type": "string"
            }
          }
        ]
      }
    },
    "/api/v1/crates/{crate}/reverse_dependencies": {
      "get": {
        "operationId": "GetCrateDependents",
        "description": "Retrieve the reverse dependencies (or dependents) of a crate.",
        "parameters": [
          {
            "name": "crate",
            "in": "path",
            "description": "Exact crate name.",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "name": "page",
            "in": "query",
            "description": "Page number (starts from 1).",
            "required": true,
            "schema": {
              "type": "number"
            }
          },
          {
            "name": "per_page",
            "in": "query",
            "description": "Page size.",
            "required": true,
            "schema": {
              "type": "number"
            }
          }
        ]
      }
    },
    "/api/v1/categories": {
      "get": {
        "operationId": "GetCategories",
        "description": "This endpoint retrieves a list of categories from the Crates.io registry.",
        "parameters": [
          {
            "name": "page",
            "in": "query",
            "description": "The page number of the results.",
            "required": false,
            "schema": {
              "type": "integer",
              "default": 1
            }
          },
          {
            "name": "per_page",
            "in": "query",
            "description": "The number of items per page.",
            "required": false,
            "schema": {
              "type": "integer",
              "default": 100
            }
          },
          {
            "name": "sort",
            "in": "query",
            "description": "The sorting order of the results, alphabetical or by crates count",
            "required": false,
            "schema": {
              "type": "string",
              "default": "alpha",
              "enum": [
                "alpha",
                "crates"
              ]
            }
          }
        ]
      }
    }
  },
  "components": {
    "schemas": {}
  }
}
```

Privacy Policy: `https://foundation.rust-lang.org/policies/privacy-policy/`