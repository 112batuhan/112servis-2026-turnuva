import type { ReactNode } from "react";
import { Link } from "react-router-dom";
import type { BracketMatch } from "../../api/types";
import { fixturePath, matchCode, UNDECIDED_TIME } from "../../lib/bracket";
import { StatusChip } from "../StatusChip";
import "./PickemPotentials.css";

interface Props {
  /** The bracket match still waiting to learn which pairing it will be. */
  match: BracketMatch;
  /** Its potential matches' rows (PickemRow), one per pairing. */
  children: ReactNode;
}

/**
 * A match whose teams hang on earlier results, as the bracket list shows it: a head line for the
 * match itself (its code, "Daha kesinleşmedi", how many pairings it may be), what that means for the
 * viewer's predictions, and each pairing branching off it as a ballot of its own.
 */
export function PickemPotentials({ match, children }: Props) {
  const code = matchCode(match);
  return (
    <li id={`pickem-${match.id}`} className="pickem-potentials">
      <div className="pickem-potentials-head">
        <Link to={fixturePath(match)} className={`pickem-potentials-code is-${match.side}`}>
          {code}
        </Link>
        <StatusChip state="potential">{match.potentials.length} olası eşleşme</StatusChip>
        <span className="pickem-potentials-when">{UNDECIDED_TIME}</span>
      </div>
      <p className="pickem-potentials-note">
        Bu maçta oynayacak takımlar daha tam belli değil. Maçta oynayabilecek bütün takım
        kombinasyonları için ayrıca tahmin girebilirsin. Maçta oynayacak takımlar belli olunca
        otomatik olarak o kombinasyona yaptığın tahmin Pick'em skoruna sayılacaktır. Diğer
        tahminlerin dikkate alınmayacaktır.
      </p>
      <ul className="pickem-potentials-list" aria-label={`${code} olası eşleşmeleri`}>
        {children}
      </ul>
    </li>
  );
}
