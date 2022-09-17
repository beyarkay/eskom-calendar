export default class NotificationService {
  private static classInstance: NotificationService;

  private constructor() {}

  public static getInstance(): NotificationService {
    debugger;
    if (!NotificationService.classInstance) {
      NotificationService.classInstance = new NotificationService();
    }

    return NotificationService.classInstance;
  }

  sendMsg(msg: string): void {
    new Notification(msg);
    debugger;
  }
}
