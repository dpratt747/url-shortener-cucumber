Feature: Shortener API
  Bef

  Scenario: Shortening a long URL
    Given I have a long URL "https://cucumber.github.io/try-cucumber-expressions/?expression=a%20boy%20named%20{string}&parameters=[]&advanced=0&step=a%20boy%20named%20%22Sue%22"
    When I make a request to the shorten URL
    Then I get a 201 status code

  Scenario: Get all shortened URLs
    Given I make 5 requests to the shorten URL endpoint
    When I make a request to get all shortened endpoints
    Then I get 6 shorten url responses