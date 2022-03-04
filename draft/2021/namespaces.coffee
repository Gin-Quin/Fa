# Namespaces in Fa are *folders*
# One file exports only *one* type or *one* value
# This way we ensure no circular dependencies are ever made

# Namespaces should be imported via:
# - import *namespace*: ... (for local namespaces)
# - use *namespace*: ... (for third party namespaces)

import /React
import /React/Core
import /React/Component

import /React/: Core, Component

import /React/:
	Core
	Component

use React/: Core, Component

use React/:
	Core
	Component
