# Namespaces in Fa can be *folders* or *files*

# Namespaces should be imported via:
# - import *namespace*: ... (for local namespaces)
# - use *namespace*: ... (for third party namespaces)

import /React
import /React/Core
import /React/Component

import /React: Core, Component

import /React: # import Core and Component
	Core
	Component
	
import /React.:  # import React.Core and React.Component
	Core
	Component

import /React.*  # import all react exports as a namespace

import /React/*  # import blindly all react exports

use React: Core, Component

use React:
	Core
	Component


# Import rules:
# - if a file has the same name than its folder, importing a folder will import the file
# - if a file is imported as-is, only its default value is imported

# For example, if we have this architecture:
# Node/Node.fa
export default type = ...

# Then we can import the default value of `Node/Node.fa by doing:
import Node # Look into folder "Node", find an homonyme file "Node.fa", find a default value in this file

# Sometimes it is needed that a file exports several values
# For example:
use and export MyLibrary: * # the whole library is imported and exported

# 