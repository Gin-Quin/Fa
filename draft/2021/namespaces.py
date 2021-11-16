# Namespaces in Fa are *folders*
# One file exports only *one* type or *one* value
# This way we ensure no circular dependencies are ever made

# Namespaces should be imported via:
# - import *namespace*: ... (for local namespaces)
# - use *namespace*: ... (for third party namespaces)

import /React/Core
import /React/Component

import /React: Core, Component
import /React/[Core, Component] # prefer this one

import /React:
	Core
	Component
import /React/:
	Core
	Component
import /React/.. # prefer this one
	Core
	Component
import /React/...
	Core
	Component

use React: Core, Component

use React:
	Core
	Component
