import SwiftUI

struct BetaTooltip: View {
    // Localized content
    struct LocalizedContent {
        let countdown: String
        let wip: String
        
        static var current: LocalizedContent {
            // Get current locale
            let locale = Locale.current.languageCode ?? "en"
            
            switch locale {
            case "fr":
                return LocalizedContent(
                    countdown: "Talers est disponible dans",
                    wip: "Inscris-toi pour être un des premiers à essayer Talers !"
                )
            default:
                return LocalizedContent(
                    countdown: "Talers is available in",
                    wip: "Sign up to be one of the first to try Talers!"
                )
            }
        }
    }
    
    // Beta date from the original component
    private let betaDate: Date
    private let content = LocalizedContent.current
    
    init(betaDate: Date) {
        self.betaDate = betaDate
    }
    
    var body: some View {
        TooltipView(color: .secondary) {
            VStack(spacing: 8) {
                Text(content.countdown)
                    .padding(.top, 4)
                
                CountdownView(to: betaDate, variant: .small)
                    .padding(.vertical, 8)
                
                Text(content.wip)
                    .padding(.bottom, 4)
            }
            .padding(4)
        }
    }
}
