export class ObjectiveChecklist {
  private container: HTMLElement;
  private titleEl: HTMLElement;
  private progressBadge: HTMLElement;
  private taskListEl: HTMLElement;
  private taskTextEl: HTMLElement;
  private checkIconEl: HTMLElement;
  private btnMenu: HTMLButtonElement;

  private onMenuClickCb: (() => void) | null = null;

  constructor() {
    this.container = document.createElement('div');
    this.container.id = 'tutorial-objective-dock';
    this.container.className = 'objective-dock glass-panel';
    this.container.style.display = 'none';

    const header = document.createElement('div');
    header.className = 'objective-header';

    const headerLeft = document.createElement('div');
    headerLeft.className = 'objective-header-left';

    const icon = document.createElement('span');
    icon.className = 'objective-sigil';
    icon.textContent = '📜';

    this.titleEl = document.createElement('h3');
    this.titleEl.className = 'objective-title';
    this.titleEl.textContent = 'Trial Lesson';

    headerLeft.appendChild(icon);
    headerLeft.appendChild(this.titleEl);

    const headerRight = document.createElement('div');
    headerRight.className = 'objective-header-right';

    this.progressBadge = document.createElement('span');
    this.progressBadge.className = 'objective-badge';
    this.progressBadge.textContent = 'Step 1/3';

    this.btnMenu = document.createElement('button');
    this.btnMenu.className = 'btn-dock-menu';
    this.btnMenu.textContent = '⚙️ [Esc]';
    this.btnMenu.title = 'Tutorial Menu (Esc)';
    this.btnMenu.addEventListener('click', () => {
      if (this.onMenuClickCb) this.onMenuClickCb();
    });

    headerRight.appendChild(this.progressBadge);
    headerRight.appendChild(this.btnMenu);

    header.appendChild(headerLeft);
    header.appendChild(headerRight);

    this.taskListEl = document.createElement('div');
    this.taskListEl.className = 'objective-task-box';

    this.checkIconEl = document.createElement('div');
    this.checkIconEl.className = 'task-status-ring pulsing';

    this.taskTextEl = document.createElement('div');
    this.taskTextEl.className = 'task-text';
    this.taskTextEl.textContent = 'Awaiting orders...';

    this.taskListEl.appendChild(this.checkIconEl);
    this.taskListEl.appendChild(this.taskTextEl);

    this.container.appendChild(header);
    this.container.appendChild(this.taskListEl);

    document.body.appendChild(this.container);
  }

  public setOnMenuClick(cb: () => void) {
    this.onMenuClickCb = cb;
  }

  public updateLesson(lessonTitle: string, stepIndex: number, totalSteps: number, taskText: string) {
    this.container.style.display = 'flex';
    this.titleEl.textContent = lessonTitle;
    this.progressBadge.textContent = `Step ${stepIndex + 1}/${totalSteps}`;
    this.taskTextEl.textContent = taskText;

    this.checkIconEl.className = 'task-status-ring pulsing';
  }

  public markCompleted() {
    this.checkIconEl.className = 'task-status-ring completed';
    this.taskTextEl.style.textDecoration = 'line-through';
    this.taskTextEl.style.color = '#00e676';
  }

  public show() {
    this.container.style.display = 'flex';
  }

  public hide() {
    this.container.style.display = 'none';
  }
}
