import { InputGateConfig, SoftFailRule } from './ScenarioLoader';

export interface GateValidationResult {
  allowed: boolean;
  errorCode?: string;
  errorMessage?: string;
}

export class InputGateFilter {
  private currentGate: InputGateConfig | null = null;
  private currentRules: SoftFailRule[] = [];

  public setGate(gate?: InputGateConfig, rules: SoftFailRule[] = []) {
    this.currentGate = gate || null;
    this.currentRules = rules;
  }

  public validateMove(unitId: number, targetQ: number, targetR: number): GateValidationResult {
    if (!this.currentGate) {
      return { allowed: true };
    }

    if (
      this.currentGate.allowed_unit_ids.length > 0 &&
      !this.currentGate.allowed_unit_ids.includes(unitId)
    ) {
      return {
        allowed: false,
        errorCode: 'ERR_INVALID_UNIT',
        errorMessage: this.findExplanation('ERR_INVALID_UNIT', 'Select your designated hero.'),
      };
    }

    if (
      this.currentGate.allowed_actions.length > 0 &&
      !this.currentGate.allowed_actions.some(a => a.toLowerCase() === 'move')
    ) {
      return {
        allowed: false,
        errorCode: 'ERR_ACTION_NOT_ALLOWED',
        errorMessage: this.findExplanation('ERR_ACTION_NOT_ALLOWED', 'Movement is not allowed in this step.'),
      };
    }

    if (this.currentGate.allowed_hex_targets.length > 0) {
      const match = this.currentGate.allowed_hex_targets.some(
        h => h.q === targetQ && h.r === targetR
      );
      if (!match) {
        return {
          allowed: false,
          errorCode: 'ERR_INVALID_TARGET',
          errorMessage: this.findExplanation('ERR_INVALID_TARGET', 'Look to the pulsing glowing ring for your destination.'),
        };
      }
    }

    return { allowed: true };
  }

  public validateAttack(unitId: number, targetId: number): GateValidationResult {
    if (!this.currentGate) {
      return { allowed: true };
    }

    if (
      this.currentGate.allowed_unit_ids.length > 0 &&
      !this.currentGate.allowed_unit_ids.includes(unitId)
    ) {
      return {
        allowed: false,
        errorCode: 'ERR_INVALID_UNIT',
        errorMessage: this.findExplanation('ERR_INVALID_UNIT', 'Command your designated champion.'),
      };
    }

    if (
      this.currentGate.allowed_actions.length > 0 &&
      !this.currentGate.allowed_actions.some(a => a.toLowerCase() === 'attack')
    ) {
      return {
        allowed: false,
        errorCode: 'ERR_ACTION_NOT_ALLOWED',
        errorMessage: this.findExplanation('ERR_ACTION_NOT_ALLOWED', 'Attacking is not permitted in this step.'),
      };
    }

    return { allowed: true };
  }

  public findExplanation(code: string, fallback: string): string {
    const rule = this.currentRules.find(r => r.error_code === code);
    if (rule) return rule.archmage_response;
    switch (code) {
      case 'ERR_OCCUPIED_HEX':
        return 'The grid admits no crowding. Two units cannot occupy one stone; chart your course around.';
      case 'ERR_INSUFFICIENT_AP':
        return "Your hero's breath is spent for this round. You have 3 AP each turn — pace your advance.";
      case 'ERR_OUT_OF_RANGE':
        return 'Your bowstring cannot reach across such distances. Advance closer before drawing.';
      case 'ERR_INVALID_TARGET':
        return 'Focus your sight on the indicated objective.';
      default:
        return fallback;
    }
  }
}
