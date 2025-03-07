- Modal {
	height = auto
	scrollable = yes
	width = 1200
	name = "SummaryCardCreateModal"
	class = "o-summaryCardCreateModal"
	clickToClose = no
	onclosed = reset

	title = Fragment {
		- Stack {
			class = "o-summaryCardCreateModal__step"

			- Transition {
				name = "fade"
				mode = "out-in"

				- when form.type is {
					null => SummaryCardTypeSelector {
						key = "target"
						bind value = form.type
					}
					"lease" => SummaryCardCreateForm {
						type = form.type
						rentalMarketingId
						rentalMarketingParticipationId
						rentalMarketing
						companyId
						participations
						cancel
						done
					}
					else => SummaryCardCreateFomMandate {
						type = form.type
						rentalMarketingId
						rentalMarketingParticipationId
						rentalMarketing
						companyId
						participations
						cancel
						done
					}
				}
			}
		}
	}
}