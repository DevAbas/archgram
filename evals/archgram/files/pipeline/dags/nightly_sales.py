from datetime import datetime

from airflow.decorators import dag, task

from etl import clean, extract, load


@dag(schedule="0 2 * * *", start_date=datetime(2026, 1, 1), catchup=False)
def nightly_sales():
    @task
    def gather():
        extract.run()

    @task
    def check():
        clean.run()

    @task
    def store():
        load.run()

    gather() >> check() >> store()


nightly_sales()
