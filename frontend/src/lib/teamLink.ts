/** The id of a team's card, so other pages can link straight to it. */
export const teamAnchorId = (teamId: string) => `team-${teamId}`;

/** The team list, scrolled to the team. */
export const teamListPath = (teamId: string) => `/teams#${teamAnchorId(teamId)}`;
