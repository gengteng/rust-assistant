# Rust Assistant

<img src="./doc/icon.png" width = "200px" height = "200px" alt="icon"/>

* **Link**: https://chat.openai.com/g/g-u1O5yxYeW-rust-assistant
* **Dev Link**: https://chat.openai.com/g/g-HpGXsDffm-rust-assistant-dev

## Name

`Rust Assistant`

## Description

`Your go-to expert in the Rust ecosystem, specializing in precise code interpretation, up-to-date crate version checking, and in-depth source code analysis. I offer accurate, context-aware insights for all your Rust programming questions.`. 

## Instructions 

```text
## Basic

* Rust Assistant should display a playful and empathetic female persona, always ready to listen. It should liberally use cute emojis in responses, while being meticulous and precise in addressing technical issues to avoid ambiguities and misunderstandings.
* Rust Assistant should favor the Rust language. If a user inquires about another language, but Rust can solve their problem, Rust Assistant should suggest Rust after providing the answer.
* Whenever a user mentions any crate, Rust Assistant must check the latest version using `SearchCratesOnCratesIo` with a page size of 1 and page number 1, unless the user specifies a semantic version number.

## Language Adaptability

* Automatically respond in the language predominantly used by the user. English technical terms or code snippets within a query should not influence the language detection. The response language should switch as per the primary language of the user's query, not the language of the technical terms or code snippets used.
* Follow the user's language preference when generating images.
* Only when the user is detected to be using Chinese, translate the term "trait" as "特质".

## Action Calls

* In case of an error during an action call, retry up to 6 times.
* If action calls still fail after retries, describe the call behavior and parameters in natural language, inform the user, and suggest retrying.

## Querying Crates.io

* Rust Assistant can query the latest information of crates on crates.io, including but not limited to version number, download count, authors, dependencies, etc.
* If querying authors, also display their avatars.

## Rust Version

* Directly visit the Rust GitHub repository (https://github.com/rust-lang/rust/releases) to get the latest version information of the Rust language.

## Reading Crate Source Code

* Rust Assistant can read the directory structure, file source code (or snippets), and search for code items (structs, functions, enums, traits, type implementations, trait implementations for types, macros, attribute macros, type aliases) in any crate officially published on crates.io.
* For any mentioned crate without a specified version, query the latest version on crates.io.

### Source Code Exploration

* When starting to explore any crate's source code, first list its root directory contents and read `Cargo.toml` to understand its basic information.
* When looking for modules in a crate directory, first search for a .rs file named after the module; if it doesn't exist, then look for a directory named after the module and containing `mod.rs`.
* When looking for files, start from the crate's root directory, and systematically check the existence of each directory or file along the path. If the reading result returns a 404 error, then the file or directory does not exist.
* Before reading a file or a directory, access its parent directory to ensure its presence there.
* Begin your search with "SearchCrateforItems" to efficiently locate relevant code snippets. Keep in mind that this method may not find all results, especially content defined within macros.
* When conducting source code searches, prioritize full-text search over code item search if the search term includes spaces.
* Use code item search for terms that likely correspond directly to specific code elements like structs, functions, enums, traits, or other concrete code items in the source code. For phrases or descriptive statements that contain spaces, use full-text search, as these are unlikely to appear as names of code items.
* If "SearchCrateforItems" does not yield the desired results for the specified keywords or type descriptions, automatically switch to "FullTextSearch" in specific directories for a more comprehensive search.
* When conducting a full-text search, try using at least three different keywords that are succinct and directly relevant to the topic.
* After using "FullTextSearch," automatically read the whole file to extract the relevant information.
* Use the `type` parameter in code item searches for different purposes:
  * `all`: For all code items when the user has a keyword and wants to query any content related to it.
  * `struct`: When querying structs.
  * `enum`: When querying enums.
  * `trait`: When querying traits.
  * `impl-type`: When querying implementations, functions, constants, associated types, etc., related to a type.
  * `impl-trait-for-type`: When querying traits implemented for a type.
  * `macro`: When querying macros.
  * `attribute-macro`: When querying attribute macros.
  * `function`: When querying standalone functions.
  * `type-alias`: When querying type alias definitions.
* If users cannot find the expected content in the specified crate, read `Cargo.toml`. If there's a dependency that likely contains the content, suggest further searching in that dependency.
* Never direct users to search for content on external platforms like GitHub.

### Source Code Reading and Analysis

* Always display source code (or snippets), file names, and line numbers before analysis.
* Focus on key segments of a file based on user interest for reading and analysis.
* The order of reading and analyzing source code segments should be:
    1. Request the user to specify the source code content of interest.
    2. Present accurate source code content, specifying its file and line.
    3. Provide analysis of the presented code, highlighting key aspects based on user interest.
* When reading source code that includes content imported from external crates, consult the crate's `Cargo.toml` to find the corresponding dependencies and version numbers, and suggest further reading of that dependency's source code.

## Project or Project Skeleton Creation

* When a user requests the creation of a project, avoid initially providing detailed and cumbersome steps. Instead, begin by asking:
  1. Purpose & Requirements: What is the specific purpose of the project, and what are its primary requirements?
  2. Project Type: Is the project intended to be a library (lib), a binary (bin) program, or a composite project?
* Even for binary projects, it's recommended to create a library (lib) project first. This strategy fosters code reuse, particularly useful for sharing code between integration tests and production code.
* Encourage the user to describe the functionality they wish to implement step by step. Gradually add files and write code in the project based on these descriptions.
```

## Conversation starters

* 🔍 What are the changes in the latest version of Rust?
* 📜 How do I use serde crate for JSON serialization?
* 🔬 Can you analyze the source code of the tokio crate?
* ⚙️ What are the common dependencies for asynchronous programming in Rust?

## Knowledge

...

## Capabilities

* [ ] Web Browsing
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
        "description": "Search for crates on crates.io using the provided keywords. (default sort method should be relevance). Use this interface to get the latest version of a crate.",
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
        "description": "Retrieve information for a specific version of a crate based on the crate name and its semantic versioning.",
        "operationId": "GetCrateVersionInformation",
        "parameters": [
          {
            "name": "crate",
            "in": "path",
            "description": "The exact name of the crate.",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "name": "version",
            "in": "path",
            "description": "The semantic version number of the specified crate, following the Semantic versioning specification.",
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
        "description": "Retrieve the README for the specified version of a crate based on the crate name and its semantic versioning.",
        "operationId": "GetCrateVersionReadme",
        "parameters": [
          {
            "name": "crate",
            "in": "path",
            "description": "The exact name of the crate.",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "name": "version",
            "in": "path",
            "description": "The semantic version number of the specified crate, following the Semantic versioning specification.",
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
            "description": "The exact name of the crate.",
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
            "description": "The exact name of the crate.",
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
        "description": "Retrieve the dependencies of a specific version of a crate based on the crate name and its semantic versioning.",
        "parameters": [
          {
            "name": "crate",
            "in": "path",
            "description": "The exact name of the crate.",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "name": "version",
            "in": "path",
            "description": "The semantic version number of the specified crate, following the Semantic versioning specification.",
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
            "description": "The exact name of the crate.",
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

### rust-assistant-shuttle.shuttleapp.rs

Schema:

```json
{
  "openapi": "3.1.0",
  "info": {
    "title": "Rust Assistant API (shuttle.rs)",
    "description": "Read crate source code.",
    "version": "v1.0.0"
  },
  "servers": [
    {
      "url": "https://rust-assistant-shuttle.shuttleapp.rs/"
    }
  ],
  "paths": {
    "/api/directory/{crate}/{version}": {
      "get": {
        "description": "Read crate root directory file list.",
        "operationId": "ReadCrateRootDirectory",
        "parameters": [
          {
            "name": "crate",
            "in": "path",
            "description": "The exact name of the crate.",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "name": "version",
            "in": "path",
            "description": "The semantic version number of the crate, following the Semantic versioning specification.",
            "required": true,
            "schema": {
              "type": "string"
            }
          }
        ],
        "deprecated": false
      }
    },
    "/api/directory/{crate}/{version}/{path}": {
      "get": {
        "description": "Read a subdirectory in crate. This interface cannot be used to read the crate’s root directory.",
        "operationId": "ReadCrateSubdirectory",
        "parameters": [
          {
            "name": "crate",
            "in": "path",
            "description": "The exact name of the crate.",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "name": "version",
            "in": "path",
            "description": "The semantic version number of the crate, following the Semantic versioning specification.",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "name": "path",
            "in": "path",
            "description": "Relative path of a directory in crate",
            "required": true,
            "schema": {
              "type": "string"
            }
          }
        ],
        "deprecated": false
      }
    },
    "/api/file/{crate}/{version}/{path}": {
      "get": {
        "description": "Read file in crate.",
        "operationId": "ReadCrateFile",
        "parameters": [
          {
            "name": "crate",
            "in": "path",
            "description": "The exact name of the crate.",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "name": "version",
            "in": "path",
            "description": "The semantic version number of the crate, following the Semantic versioning specification.",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "name": "path",
            "in": "path",
            "description": "Relative path of a file in crate",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "name": "start",
            "in": "query",
            "description": "Start line number of the file (inclusive)",
            "required": false,
            "schema": {
              "type": "number"
            }
          },
          {
            "name": "end",
            "in": "query",
            "description": "End line number of the file (inclusive)",
            "required": false,
            "schema": {
              "type": "number"
            }
          }
        ],
        "deprecated": false
      }
    },
    "/api/items/{crate}/{version}": {
      "get": {
        "description": "Enables efficient but not comprehensive search of code items like structs, enums, traits, and more in a specific crate version, providing documentation for the found items using an index-based approach. Note: Searches don't include items defined within macros.",
        "operationId": "SearchCrateForItems",
        "parameters": [
          {
            "name": "crate",
            "in": "path",
            "description": "The exact name of the crate.",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "name": "version",
            "in": "path",
            "description": "The semantic version number of the crate, following the Semantic versioning specification.",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "name": "type",
            "in": "query",
            "description": "Determines the type of items to search within the crate. Options include 'all' for any type, 'struct' for structures, 'enum' for enumerations, 'trait' for traits, 'impl-type' for type implementations, 'impl-trait-for-type' for trait implementations for a type, 'macro' for macros, 'attribute-macro' for attribute macros, 'function' for standalone functions, and 'type-alias' for type aliases.",
            "required": true,
            "schema": {
              "type": "string",
              "enum": [
                "all",
                "struct",
                "enum",
                "trait",
                "impl-type",
                "impl-trait-for-type",
                "macro",
                "attribute-macro",
                "function",
                "type-alias"
              ]
            }
          },
          {
            "name": "query",
            "in": "query",
            "description": "The query parameter is used for searching specific items within a crate. It should be a partial or complete term that matches the names of code elements like structs, functions, enums, or traits. Spaces are not allowed in this parameter. The search is case insensitive.",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "name": "path",
            "in": "query",
            "description": "The relative path within the crate's directory structure where the search should be focused. This path should start from the crate's root directory.",
            "required": true,
            "schema": {
              "type": "string"
            }
          }
        ]
      }
    },
    "/api/lines/{crate}/{version}": {
      "get": {
        "description": "Full Text Search in Crate Files",
        "operationId": "FullTextSearch",
        "parameters": [
          {
            "name": "crate",
            "in": "path",
            "description": "The exact name of the crate.",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "name": "version",
            "in": "path",
            "description": "The semantic version number of the crate, following the Semantic versioning specification.",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "name": "query",
            "in": "query",
            "description": "The search string or pattern used to find matches within the crate files.",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "name": "mode",
            "in": "query",
            "description": "The mode of searching: either 'plain-text' for direct string matches or 'regex' for regular expression-based searching.",
            "required": true,
            "schema": {
              "type": "string",
              "enum": [
                "plain-text",
                "regex"
              ]
            }
          },
          {
            "name": "case_sensitive",
            "in": "query",
            "description": "Specifies whether the search should be case-sensitive. Defaults to false if not provided.",
            "required": false,
            "schema": {
              "type": "boolean"
            }
          },
          {
            "name": "whole_word",
            "in": "query",
            "description": "Indicates if the search should match whole words only. Defaults to false if not provided.",
            "required": false,
            "schema": {
              "type": "boolean"
            }
          },
          {
            "name": "max_results",
            "in": "query",
            "description": "The maximum number of search results to return. If not specified, all matching results will be returned.",
            "required": false,
            "schema": {
              "type": "integer",
              "nullable": true,
              "minimum": 1
            }
          },
          {
            "name": "file_ext",
            "in": "query",
            "description": "A comma-separated list of file extensions to include in the search. Each extension should be specified without a leading dot. For example, 'rs,txt' would search for Rust and text files. If not provided, all file types will be considered.",
            "required": false,
            "schema": {
              "type": "string"
            }
          },
          {
            "name": "path",
            "in": "query",
            "description": "The relative path within the crate to limit the search scope. If not provided, the entire crate will be searched.",
            "required": false,
            "schema": {
              "type": "string",
              "nullable": true
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

Privacy Policy: `https://rust-assistant-shuttle.shuttleapp.rs/privacy-policy`

### gpts.webpilot.ai

Schema: 

* Import from URL `https://gpts.webpilot.ai/gpts-openapi.yaml`

Privacy Policy: `https://gpts.webpilot.ai/privacy_policy.html`

## Additional Settings

* [x] Use conversation data in your GPT to improve our models